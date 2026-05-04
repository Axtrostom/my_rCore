use super::{get_block_cache, BlockDevice, BLOCK_SZ};
use alloc::sync::Arc;
/// A bitmap block
type BitmapBlock = [u64; 64];//一个 位图块， 因为一个块是 512 字节， 每个 u64 是 8 字节，所以一个块可以存储 512 / 8 = 64 个 u64
/// Number of bits in a block
const BLOCK_BITS: usize = BLOCK_SZ * 8;
/// A bitmap
pub struct Bitmap {
    start_block_id: usize,//起始块 id
    blocks: usize,//bitmap自身占用的块数量
}

//将输入的 块号 转化为在 bitmap 中的 第几个 block 的第几个 u64 的第几个 bit 
fn decomposition(mut bit: usize) -> (usize, usize, usize) {
    let block_pos = bit / BLOCK_BITS;
    bit %= BLOCK_BITS;
    (block_pos, bit / 64, bit % 64)
}

impl Bitmap {
    /// A new bitmap from start block id and number of blocks
    pub fn new(start_block_id: usize, blocks: usize) -> Self {//创建
        Self {
            start_block_id,
            blocks,
        }
    }
    //在 bitmap 分配一个 bit
    pub fn alloc(&self, block_device: &Arc<dyn BlockDevice>) -> Option<usize> {
        for block_id in 0..self.blocks {//遍历所有 bitmap 块
            let pos = get_block_cache(//把当前bitmap块读到缓存中
                block_id + self.start_block_id as usize,
                Arc::clone(block_device),
            )
            .lock()//加锁，准备修改
            .modify(0, |bitmap_block: &mut BitmapBlock| {//修改，modify函数是 BlockCache 中实现的
                // 偏移量为 0，意思是将当前缓存块从第 0 个字节开始，当作 BitmapBlock 对待。 
                // 底层会生成一个指向这片内存的可变借用（&mut BitmapBlock），
                // 作为参数传递给当前的闭包，让闭包在内部自由修改这 64 个 u64 的状态。
                if let Some((bits64_pos, inner_pos)) = bitmap_block//获取 bitmap 内部 第 多少个 u64 的第几位 是 0
                    .iter()
                    .enumerate()//返回的是 (usize , &u64)
                    .find(|(_, bits64)| **bits64 != u64::MAX)//找到一个不全为1的值，即有空位的地方
                    //find 输入值应该是索引类型 ，这里是 &(usize , &u64) 所以 bits64 套了两层引用
                    .map(|(bits64_pos, bits64)| (bits64_pos, bits64.trailing_ones() as usize))
                    //trailing_ones() 是 Rust 标准库提供的一个位运算方法。它的作用是：从二进制的最右边（最低位，Least Significant Bit）开始数，统计连续出现了多少个 1
                {
                    // modify cache
                    bitmap_block[bits64_pos] |= 1u64 << inner_pos;//通过 inner_pos 修改对应位的值
                    Some(block_id * BLOCK_BITS + bits64_pos * 64 + inner_pos as usize) //返回分配的 bit 的全局 id
                    //这里是 modify 的 f 参数闭包的返回值，最后接收到 pos
                } else {
                    None
                }
            });
            if pos.is_some() {
                return pos; 
            }
        }
        None
    }//吗的，这逼函数咋这样写的，他吗的一句一句写不行吗
    /// 从 bitmap 中删除一个 bit
    pub fn dealloc(&self, block_device: &Arc<dyn BlockDevice>, bit: usize) {
        let (block_pos, bits64_pos, inner_pos) = decomposition(bit);//将第多少个块转化为 bitmap 中具体的位
        get_block_cache(block_pos + self.start_block_id, Arc::clone(block_device))//将对应的 bitmap 块读取到缓存中
            .lock()
            .modify(0, |bitmap_block: &mut BitmapBlock| {
                assert!(bitmap_block[bits64_pos] & (1u64 << inner_pos) > 0);
                bitmap_block[bits64_pos] -= 1u64 << inner_pos;
            });
    }
    /// Get the max number of allocatable blocks
    pub fn maximum(&self) -> usize {
        self.blocks * BLOCK_BITS
    }
}
