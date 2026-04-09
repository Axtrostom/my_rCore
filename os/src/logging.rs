use log::{self, Level, LevelFilter, Log, Metadata, Record};


// 1. 定义你的记录器结构体（就是一个打工的“空壳子”）
pub struct MyLogger;

// 2. 正式签订并履行 Log 契约
impl Log for MyLogger {
    
    // 【安检网关】：决定哪条日志有资格进入下一步
    fn enabled(&self, metadata: &Metadata) -> bool {
        // TODO (选做): 你可以在这里写判断逻辑。
        // 比如： metadata.level() <= Level::Info
        // 如果图省事，直接返回 true，代表所有发来的日志我们都接收处理。
        true
    }

    // 【核心车间】：真正负责排版和打印输出的地方
    fn log(&self, record: &Record) {
        // 第一步：先过一遍安检，看看这条日志够不够资格
        if self.enabled(record.metadata()) {
            
            // 第二步：提取日志的关键材料
            let level = record.level(); // 日志的级别 (Error, Warn, Info, Debug, Trace)
            let args = record.args();   // 用户真正想打印的文字内容

            // TODO (必做): 在这里施展你的魔法！
            // 1. 写一个 match 语句，根据 `level` 的不同，匹配出不同的 ANSI 颜色代码（数字）。
            // 2. 调用你之前手搓的 `println!` 宏。
            // 3. 把 颜色代码、`level`、`args` 拼装在一起打印出来。
            // (记得在打印内容的最后加上 \u{1B}[0m 来重置颜色！)
            let color = match level{
                Level::Error => 31, // Red
                Level::Warn => 93,  // BrightYellow
                Level::Info => 34,  // Blue
                Level::Debug => 32, // Green
                Level::Trace => 90, // BrightBlack
            };
            println!(
                "\u{1B}[{}m[{:>5}] {}\u{1B}[0m",
                color,
                record.level(),
                record.args(),
            );
            
        }
    }

    // 【下班清理】：冲刷缓冲区
    fn flush(&self) {
        // TODO: 我们的裸机 OS 是实时输出到串口的，没有硬盘缓存机制。
        // 所以这里什么都不用写，保持空大括号即可！
    }
}

// 3. 启动引擎（全局初始化）
pub fn init() {
    static LOGGER: MyLogger = MyLogger;
    
    // 把我们写好的空壳子挂载到全局
    log::set_logger(&LOGGER).unwrap();
    
    // TODO (选做): 在这里设置一个全局的最大放行级别。
    // 你可以写死，比如 log::set_max_level(log::LevelFilter::Info);
    // 也可以去挑战一下读取 option_env!("LOG") 环境变量的高级玩法！
    log::set_max_level(match option_env!("LOG") {
        Some("ERROR") => LevelFilter::Error,
        Some("WARN") => LevelFilter::Warn,
        Some("INFO") => LevelFilter::Info,
        Some("DEBUG") => LevelFilter::Debug,
        Some("TRACE") => LevelFilter::Trace,
        _ => LevelFilter::Info,
    });

}