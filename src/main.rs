#![no_main]
#![no_std]
#![feature(panic_info_message)]

#[macro_use]
mod console;
//#[macro_use] 的准确定义是：将紧接着的模块（mod）或外部库（crate）中定义的宏（Macro，即调用时必须带 ! 的代码生成器）释放并广播出来，让它们能够在当前文件（甚至整个项目中）直接被使用。


mod lang_items;
mod sbi;



use core::arch::global_asm;
//下面这行意思说是，直接把这个 entry.asm 拿过来，但是不用编译器去编译它，而是直接把它当成汇编代码来处理
global_asm!(include_str!("entry.asm"));


#[unsafe(no_mangle)]
pub fn rust_main() -> ! {
    clear_bss();

    println!("hello , {} 123!" , "world");

    panic!("Shutdown machine!");
}

fn clear_bss() {
    unsafe extern "C" {
        fn sbss();
        fn ebss();
    }
    let start = sbss as usize;
    let end = ebss as usize;

    // for addr in start..end{
    //     unsafe { (a as *mut u8).write_volatile(0) }
    // }

    (start..end).for_each(|a| {unsafe { (a as *mut u8).write_volatile(0) }});
}