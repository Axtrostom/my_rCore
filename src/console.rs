use crate::sbi::console_putchar;
use core::fmt::{self, Write};

struct Stdout;

impl Write for Stdout {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for c in s.chars() {
            console_putchar(c as usize);
        }
        Ok(())
    }
}

pub fn print(args: fmt::Arguments) {
    Stdout.write_fmt(args).unwrap();
}

#[macro_export]
macro_rules! print {
    ($fmt: literal $(, $($arg: tt)+)?) => {
        $crate::console::print(format_args!($fmt $(, $($arg)+)?));
    }
}

/*
这个宏可以理解成怎么讲一个 字符串 正则转化为一个 fmt::Arguments 数据类型并且传送给 print
fmt::Arguments 是什么？
这是 Rust 编译器内部使用的一个极其特殊的黑盒数据结构。
当你写下 println!("Hello {} {}!", "World", 123) 时，Rust 编译器在底层会把它打包成一个 fmt::Arguments 结构体。这个结构体里安全地封存了：
字符串模板："Hello {} {}!"
待填入的变量引用："World" 和 123
它保证了类型安全，让你不可能把整数错当成字符串打印出来。

简单来说就是对 "Hello {} {}!", "World", 123 的处理

输入是 ($fmt: literal $(, $($arg: tt)+)?)
每个 $ 代表一个要匹配的变量
比如说 $fmt: literal 意思是有一个名为 fmt 的变量 ，要求属性为 literal ， 对应前面例子中的  "Hello {} {}!"
后面开始处理后面的参数
$() 这种是什么意思？
在宏的语法里，$() 代表的是“循环与重复（重复捕获）


$(, $($arg: tt)+)?

$arg: tt
tt：全称是 Token Tree（语法树节点）。在 Rust 宏的眼里，一段代码里的变量名 a、数字 123、甚至是一个表达式 1 + 1，都是一个完整的 tt

$() +：这里的加号 + 代表**“一次或多次”**。

$( ,  ...  )?
$() ?：这里的问号 ? 代表**“零次或一次”（即可选的）



$crate是什么意思？它是一个超级指南针，永远指向你的项目的“根目录”。
根据输入进行匹配 ，塞入接下来的 print （print 是什么？）byd print 是前面实现的函数
format_args! 是什么？另外一个宏？另外一个宏！在编译期把字符串模板和零散的变量打包、缝合，最终生成那个叫 fmt::Arguments 的黑盒结构




*/




#[macro_export]
macro_rules! println {
    ($fmt: literal $(, $($arg: tt)+)?) => {
        $crate::console::print(format_args!(concat!($fmt, "\n") $(, $($arg)+)?));
    }
}