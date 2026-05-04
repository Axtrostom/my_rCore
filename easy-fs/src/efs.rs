use super::{
    block_cache_sync_all, get_block_cache, Bitmap, BlockDevice, DiskInode, DiskInodeType, Inode,
    SuperBlock,
};
use crate::BLOCK_SZ;
use alloc::sync::Arc;
use spin::Mutex;
///An easy file system on block
pub struct EasyFileSystem {
    ///Real device
    pub block_device: Arc<dyn BlockDevice>,//块设备
    ///Inode bitmap
    pub inode_bitmap: Bitmap,//索引节点位图
    ///Data bitmap
    pub data_bitmap: Bitmap,//数据块位图
    inode_area_start_block: u32,//索引区域起始块号
    data_area_start_block: u32,//数据节点起始块号
}

type DataBlock = [u8; BLOCK_SZ];
/// An easy fs over a block device
impl EasyFileSystem {
    /// A data block of block size
    pub fn create(
        block_device: Arc<dyn BlockDevice>,//块设备
        total_blocks: u32,//总块数量
        inode_bitmap_blocks: u32,//索引位图块数
    ) -> Arc<Mutex<Self>> {
        // calculate block size of areas & create bitmaps
        let inode_bitmap = Bitmap::new(1, inode_bitmap_blocks as usize);//创建位图
        let inode_num = inode_bitmap.maximum();//根据索引位图确定索引节点数量
        let inode_area_blocks =
            ((inode_num * core::mem::size_of::<DiskInode>() + BLOCK_SZ - 1) / BLOCK_SZ) as u32;//计算索引块数量
        let inode_total_blocks = inode_bitmap_blocks + inode_area_blocks;//计算加上位图块，索引部分总占的数量
        let data_total_blocks = total_blocks - 1 - inode_total_blocks;//计算数据块的总量
        let data_bitmap_blocks = (data_total_blocks + 4096) / 4097;//计算数据位图要占的块数
        let data_area_blocks = data_total_blocks - data_bitmap_blocks;//数据区 块数
        let data_bitmap = Bitmap::new(//创建数据 bitmap
            (1 + inode_total_blocks) as usize,
            data_bitmap_blocks as usize,
        );
        let mut efs = Self {
            block_device: Arc::clone(&block_device),
            inode_bitmap,
            data_bitmap,
            inode_area_start_block: 1 + inode_bitmap_blocks,
            data_area_start_block: 1 + inode_total_blocks + data_bitmap_blocks,
        };//创建 文件系统
        // clear all blocks
        for i in 0..total_blocks {//遍历所有的块
            get_block_cache(i as usize, Arc::clone(&block_device))//读取相应的块，获得可变引用
                .lock()
                .modify(0, |data_block: &mut DataBlock| {
                    for byte in data_block.iter_mut() {//将数据块中所有数据设置成 0
                        *byte = 0;
                    }
                });
        }
        // initialize SuperBlock
        get_block_cache(0, Arc::clone(&block_device)).lock().modify(
            0,
            |super_block: &mut SuperBlock| {//创建超级块
                super_block.initialize(
                    total_blocks,
                    inode_bitmap_blocks,
                    inode_area_blocks,
                    data_bitmap_blocks,
                    data_area_blocks,
                );
            },
        );
        // write back immediately
        // create a inode for root node "/"
        assert_eq!(efs.alloc_inode(), 0);//在 索引节点 bitmap 中分配一个 bit，分配给根节点
        let (root_inode_block_id, root_inode_offset) = efs.get_disk_inode_pos(0);
        get_block_cache(root_inode_block_id as usize, Arc::clone(&block_device))
            .lock()
            .modify(root_inode_offset, |disk_inode: &mut DiskInode| {
                disk_inode.initialize(DiskInodeType::Directory);
            });
        block_cache_sync_all();//将这个修改同步到 块设备
        Arc::new(Mutex::new(efs))//用 Arc 包裹
    }
    /// Open a block device as a filesystem
    /// 从磁盘读取文件系统
    pub fn open(block_device: Arc<dyn BlockDevice>) -> Arc<Mutex<Self>> {
        // read SuperBlock
        get_block_cache(0, Arc::clone(&block_device))//读取
            .lock()
            .read(0, |super_block: &SuperBlock| {//作为一个 SuperBlock 读取进来
                assert!(super_block.is_valid(), "Error loading EFS!");//验证 超级块 有效性
                let inode_total_blocks =
                    super_block.inode_bitmap_blocks + super_block.inode_area_blocks;//计算 Inode 相关区域（位图区域 + 索引节点区域）占据的总块数
                let efs = Self {// 根据超级块的元数据，在内存中恢复（挂载）文件系统管理器实例
                    block_device,
                    inode_bitmap: Bitmap::new(1, super_block.inode_bitmap_blocks as usize),
                    data_bitmap: Bitmap::new(
                        (1 + inode_total_blocks) as usize,
                        super_block.data_bitmap_blocks as usize,
                    ),
                    inode_area_start_block: 1 + super_block.inode_bitmap_blocks,// 索引块起始位置
                    data_area_start_block: 1 + inode_total_blocks + super_block.data_bitmap_blocks,// 数据块起始位置
                };
                Arc::new(Mutex::new(efs))
            })
    }
    /// Get the root inode of the filesystem
    pub fn root_inode(efs: &Arc<Mutex<Self>>) -> Inode {
        let block_device = Arc::clone(&efs.lock().block_device);
        // acquire efs lock temporarily
        let (block_id, block_offset) = efs.lock().get_disk_inode_pos(0);
        // release efs lock
        Inode::new(block_id, block_offset, Arc::clone(efs), block_device)
    }
    /// Get inode by id
    /// 计算某个 Inode 在磁盘上的绝对物理位置
    /// 输入的是 inode id ，输出 u32 块号 和 usize 内部偏移
    pub fn get_disk_inode_pos(&self, inode_id: u32) -> (u32, usize) {
        let inode_size = core::mem::size_of::<DiskInode>();
        let inodes_per_block = (BLOCK_SZ / inode_size) as u32;
        let block_id = self.inode_area_start_block + inode_id / inodes_per_block;
        (
            block_id,
            (inode_id % inodes_per_block) as usize * inode_size,
        )
    }
    /// Get data block by id
    pub fn get_data_block_id(&self, data_block_id: u32) -> u32 {
        self.data_area_start_block + data_block_id
    }
    /// Allocate a new inode
    pub fn alloc_inode(&mut self) -> u32 {
        self.inode_bitmap.alloc(&self.block_device).unwrap() as u32
    }

    /// Allocate a data block
    pub fn alloc_data(&mut self) -> u32 {
        self.data_bitmap.alloc(&self.block_device).unwrap() as u32 + self.data_area_start_block
    }
    /// Deallocate a data block
    pub fn dealloc_data(&mut self, block_id: u32) {
        get_block_cache(block_id as usize, Arc::clone(&self.block_device))
            .lock()
            .modify(0, |data_block: &mut DataBlock| {
                data_block.iter_mut().for_each(|p| {
                    *p = 0;
                })
            });
        self.data_bitmap.dealloc(
            &self.block_device,
            (block_id - self.data_area_start_block) as usize,
        )
    }
}
