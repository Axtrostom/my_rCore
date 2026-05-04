use super::{BlockDevice, BLOCK_SZ};
use alloc::boxed::Box;
use alloc::collections::VecDeque;
use alloc::sync::Arc;
use core::alloc::Layout;
use core::mem::ManuallyDrop;
use core::ptr::{addr_of, addr_of_mut};
use core::slice;
use lazy_static::*;
use spin::Mutex;

/// Use `ManuallyDrop` to ensure data is deallocated with an alignment of `BLOCK_SZ`
/// 声明了一个 BLOCK_SZ 字节大小的空间用来缓存
/// ManuallyDrop 作用是告诉编译器，当这个变量离开作用域时，绝对不要自动调用它的析构函数（Drop）。
struct CacheData(ManuallyDrop<Box<[u8; BLOCK_SZ]>>);

impl CacheData {
    pub fn new() -> Self {
        let data = unsafe {
            //alloc::alloc::alloc是通过 Layout 来分配相应的内存空间
            //返回值是指向分配的内存的指针
            let raw = alloc::alloc::alloc(Self::layout());
            Box::from_raw(raw as *mut [u8; BLOCK_SZ])//将 raw 转换为一个指向 [u8; BLOCK_SZ] 的指针，并将其包装在一个 Box 中
        };
        Self(ManuallyDrop::new(data))
    }

    fn layout() -> Layout {
        //layout 是用来描述内存块的大小和对齐方式的结构体
        //包括 内存大小 和 对齐要求
        Layout::from_size_align(BLOCK_SZ, BLOCK_SZ).unwrap()
    }
}

impl Drop for CacheData {
    //销毁 
    fn drop(&mut self) {
        let ptr = self.0.as_mut_ptr();
        unsafe { alloc::alloc::dealloc(ptr, Self::layout()) };//释放内存空间
    }
}

impl AsRef<[u8]> for CacheData {
    //提供一个只读的字节切片 &[u8]
    fn as_ref(&self) -> &[u8] {
        let ptr = self.0.as_ptr() as *const u8;
        unsafe { slice::from_raw_parts(ptr, BLOCK_SZ) }
    }
}

impl AsMut<[u8]> for CacheData {
    //提供一个可变的字节切片 &mut [u8]
    fn as_mut(&mut self) -> &mut [u8] {
        let ptr = self.0.as_mut_ptr() as *mut u8;
        unsafe { slice::from_raw_parts_mut(ptr, BLOCK_SZ) }
    }
}

/// Cached block inside memory
pub struct BlockCache {
    /// 缓存数据，缓存数据区域
    cache: CacheData,
    /// 对应的块 id 
    block_id: usize,
    /// 对应的块设备，只要是实现了 `BlockDevice` trait 的类型都可以
    block_device: Arc<dyn BlockDevice>,
    /// 是否修改
    modified: bool,
}

impl BlockCache {
    ///创建blockcache ，同时从块设备中读取数据到缓存中
    pub fn new(block_id: usize, block_device: Arc<dyn BlockDevice>) -> Self {
        let mut cache = CacheData::new();//创建对应的缓存区域
        block_device.read_block(block_id, cache.as_mut());//根据 blockID 将数据读取到 cache 中
        Self {
            cache,
            block_id,
            block_device,
            modified: false,
        }
    }
    ///得到在缓存中对应偏移量的地址
    /// self.cache 在内存中只是一个毫无语义的字节数组 [u8; 512]。但是，文件系统在磁盘上存储的并不是毫无意义的字节，而是具有严格内存布局的数据结构体。例如：
        // 第 0 字节开始，可能存放的是一个 SuperBlock 结构体。
        // 第 64 字节开始，可能存放的是一个 DiskInode 结构体。
        // 第 128 字节开始，可能存放的是一个 DirEntry 结构体。
    fn addr_of_offset(&self, offset: usize) -> *const u8 {
        addr_of!(self.cache.as_ref()[offset])
    }

    fn addr_of_offset_mut(&mut self, offset: usize) -> *mut u8 {
        addr_of_mut!(self.cache.as_mut()[offset])
    }

    //得到在缓存中对应偏移量的地址，并将其转换为一个类型为 T 的引用 
    pub fn get_ref<T>(&self, offset: usize) -> &T
    where
        T: Sized,//要求 T 实现了 Sized trait，表示 T 的大小在编译时是已知的
    {
        let type_size = core::mem::size_of::<T>();
        assert!(offset + type_size <= BLOCK_SZ);
        let addr = self.addr_of_offset(offset) as *const T;
        unsafe { &*addr }
    }
    //一样，但是是得到可变引用
    pub fn get_mut<T>(&mut self, offset: usize) -> &mut T
    where
        T: Sized,
    {
        let type_size = core::mem::size_of::<T>();
        assert!(offset + type_size <= BLOCK_SZ);
        self.modified = true;
        let addr = self.addr_of_offset_mut(offset) as *mut T;
        unsafe { &mut *addr }
    }
    //最终暴露给外部的接口
    // 这个接口的作用是让外部代码能够以一种类型安全的方式访问缓存中的数据。
    //通过传入一个偏移量和一个闭包，外部代码可以获取到缓存中对应位置的数据，并对其进行操作，而不需要直接处理原始的字节数组。
    //FnOnce 是一个函数 trait，表示在当前的read函数里面，只使用一次 f
    //整个函数的作用是，从偏移量位置开始，读取一个类型为 T 的数据
    //然后将 T 的不可变引用 &T 作为参数传递给 f ，最终返回 V
    pub fn read<T, V>(&self, offset: usize, f: impl FnOnce(&T) -> V) -> V {
        f(self.get_ref(offset))
    }

    //这个跟 read 相同，不同的是获取的是可变引用 &mut T
    pub fn modify<T, V>(&mut self, offset: usize, f: impl FnOnce(&mut T) -> V) -> V {
        f(self.get_mut(offset))
    }

    //将缓存中的数据写回到块设备中
    //同步缓存和块设备的数据
    pub fn sync(&mut self) {
        if self.modified {
            self.modified = false;
            self.block_device
                .write_block(self.block_id, self.cache.as_ref());
        }
    }
}

impl Drop for BlockCache {
    fn drop(&mut self) {
        self.sync()
    }
}
/// Use a block cache of 16 blocks
const BLOCK_CACHE_SIZE: usize = 16;

//块缓存管理器，负责管理内存中的块缓存
pub struct BlockCacheManager {
    //使用 VecDeque 来存储块 id 和对应的块缓存
    queue: VecDeque<(usize, Arc<Mutex<BlockCache>>)>,
}

impl BlockCacheManager {
    pub fn new() -> Self {
        Self {
            queue: VecDeque::new(),
        }
    }

    //读取 某一个块设备 的 block_id 块
    //要是当前不在缓存内，则读取到缓存中
    pub fn get_block_cache(
        &mut self,
        block_id: usize,
        block_device: Arc<dyn BlockDevice>,
    ) -> Arc<Mutex<BlockCache>> {
        if let Some(pair) = self.queue.iter().find(|pair| pair.0 == block_id) {
            //如果缓存命中
            Arc::clone(&pair.1)
        } else {
            // substitute
            //如果缓存未命中
            if self.queue.len() == BLOCK_CACHE_SIZE {//缓存满了
                // from front to tail
                if let Some((idx, _)) = self
                    .queue
                    .iter()
                    .enumerate()
                    .find(|(_, pair)| Arc::strong_count(&pair.1) == 1)
                    //找到一引用计数为 1 的块缓存
                    //因为 BlockCacheManager 对所有的块缓存都有一个引用
                    //所以如果某个块缓存的引用计数为 1，说明没有其他代码在使用它，可以安全地将其从缓存中移除
                {
                    //idx为要被替换的块缓存在队列中的索引
                    //drain 是移除 VecDeque 中范围内的元素
                    //移除后空位会到队列的末尾
                    self.queue.drain(idx..=idx);
                    
                } else {
                    panic!("Run out of BlockCache!");
                }
            }
            // 创建一个新的块缓存，并将其添加到队列的末尾
            let block_cache = Arc::new(Mutex::new(BlockCache::new(
                block_id,
                Arc::clone(&block_device),
            )));
            self.queue.push_back((block_id, Arc::clone(&block_cache)));
            block_cache
        }
    }
}

lazy_static! {
    /// 依旧是 lazy_static! 宏来创建一个全局的块缓存管理器 BLOCK_CACHE_MANAGER
    pub static ref BLOCK_CACHE_MANAGER: Mutex<BlockCacheManager> =
        Mutex::new(BlockCacheManager::new());
}
/// Get the block cache corresponding to the given block id and block device
/// 读取某一个块设备 的 block_id 块，要是当前不在缓存内，则读取到缓存中
/// 返回一个 Arc<Mutex<BlockCache>>，表示对块缓存的共享所有权和线程安全的访问
pub fn get_block_cache(
    block_id: usize,
    block_device: Arc<dyn BlockDevice>,
) -> Arc<Mutex<BlockCache>> {
    BLOCK_CACHE_MANAGER
        .lock()
        .get_block_cache(block_id, block_device)
}
/// Sync all block cache to block device
pub fn block_cache_sync_all() {
    let manager = BLOCK_CACHE_MANAGER.lock();
    for (_, cache) in manager.queue.iter() {
        cache.lock().sync();
    }
}
