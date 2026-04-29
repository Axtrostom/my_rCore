#![no_std]
#![no_main]
#![allow(clippy::println_empty_string)]

extern crate alloc;

#[macro_use]
extern crate user_lib;

const LF: u8 = 0x0au8;//换行
const CR: u8 = 0x0du8;//回车
const DL: u8 = 0x7fu8;//删除
const BS: u8 = 0x08u8;//退格

use alloc::string::String;
use user_lib::console::getchar;
use user_lib::{exec, fork, waitpid};

#[unsafe(no_mangle)]
pub fn main() -> i32 {
    println!("Rust user shell");
    let mut line: String = String::new();
    print!(">> ");
    loop {
        let c = getchar();
        match c {
            LF | CR => {
                println!("");
                if !line.is_empty() {
                    line.push('\0');//添加字符串结束标志
                    let pid = fork();//创建子进程
                    if pid == 0 {//子进程
                        // child process
                        if exec(line.as_str()) == -1 {//执行失败
                            println!("Error when executing!");
                            return -4;
                        }
                        unreachable!();
                    } else {//父进程
                        let mut exit_code: i32 = 0;
                        let exit_pid = waitpid(pid as usize, &mut exit_code);//等待上面创建的子进程结束
                        assert_eq!(pid, exit_pid);
                        println!("Shell: Process {} exited with code {}", pid, exit_code);
                    }
                    line.clear();
                }
                print!(">> ");
            }
            BS | DL => {//删除
                if !line.is_empty() {
                    //先退格，再覆盖一个空格，最后再退格，这样就把屏幕上的字符删除了
                    ///很多人以为，在键盘上按下 Backspace（退格键），屏幕上的字符就会自动被“删掉”。错！这是现代高级操作系统给你造成的错觉。
                    ///在最底层的终端（或者打字机时代）里，BS（ASCII 码 0x08）的真实含义仅仅是：光标往左平移一格。它绝对不会主动去抹掉屏幕上的像素。
                    print!("{}", BS as char);
                    print!(" ");
                    print!("{}", BS as char);
                    line.pop();
                }
            }
            _ => {
                print!("{}", c as char);
                line.push(c as char);
            }
        }
    }
}
