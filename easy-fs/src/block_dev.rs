use core::any::Any;
/// Trait for block devices
/// which reads and writes data in the unit of blocks
/// 
/// 
/// 
/// 这个是定义了一个叫 BlockDevice 的 trait
/// 实现这个 trait 需要实现两个方法：read_block 和 write_block
/// Send + Sync + Any 这段的写法是 Supertrait ， 意思是实现了 BlockDevice 的类型也必须实现 Send、Sync 和 Any 这三个 trait
pub trait BlockDevice: Send + Sync + Any {
    ///从块设备中读取数据到缓冲区
    /// 参数分别是 块 id 和一个可变的字节切片作为缓冲区
    fn read_block(&self, block_id: usize, buf: &mut [u8]);
    ///将数据从缓冲区写入块设备
    /// 参数分别是 块 id 和一个字节切片作为缓冲区 
    fn write_block(&self, block_id: usize, buf: &[u8]);
}
