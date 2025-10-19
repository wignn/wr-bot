use sysinfo::System;

pub struct SysInfo {
    pub memory: String,
    pub cpu: String,
    pub os: String,
}

impl SysInfo {
    pub fn new() -> Self {
        let mut sys = System::new_all();
        sys.refresh_all();

        let os = System::long_os_version().unwrap_or_else(|| "Unknown OS".to_string());

        let cpu = if let Some(first_cpu) = sys.cpus().first() {
            format!("{} ({:.2}% active)", first_cpu.brand(), first_cpu.cpu_usage())
        } else {
            "Unknown CPU".to_string()
        };

        let total_mem = sys.total_memory() / 1024 / 1024;
        let used_mem = sys.used_memory() / 1024 / 1024;
        let memory = format!("{} MB / {} MB digunakan", used_mem, total_mem);

        Self { memory, cpu, os }
    }

}
