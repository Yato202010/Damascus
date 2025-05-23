// Copyright 2025 Yato202010
//
// Licensed under the Apache License, Version 2.0 (the "License"); you may not use this file except in compliance with the License. You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software distributed under the License is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied. See the License for the specific language governing permissions and limitations under the License.
#[cfg(target_os = "linux")]
#[allow(dead_code)]
mod linux;

#[cfg(target_os = "windows")]
#[allow(dead_code)]
mod windows;

use colored::Colorize;
#[cfg(target_os = "linux")]
pub use linux::register_test;

#[cfg(target_os = "windows")]
pub use windows::register_test;

type PassedCount = usize;
type FailedCount = usize;
type FilteredCount = usize;

struct TestRunner {
    tests: Vec<(String, Box<dyn Fn() + std::panic::UnwindSafe>)>,
    filtered: Vec<usize>,
    current_test: usize,
}

#[allow(dead_code)]
impl TestRunner {
    fn len(&self) -> usize {
        self.tests.len()
    }

    fn clear(&mut self) {
        self.tests.clear();
        self.filtered.clear();
        self.current_test = 0;
    }

    fn run(&mut self) -> (PassedCount, FailedCount, FilteredCount) {
        use colored::*;
        let mut passed = 0;
        let mut failed = 0;
        while self.current_test < self.tests.len() {
            let (name, func) = &self.tests[self.current_test];
            print!("test end2end::{} ... ", name);
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(func));

            if self.filtered.contains(&self.current_test) {
                println!()
            } else if result.is_ok() {
                passed += 1;
                println!("{}", "ok".green());
            } else {
                failed += 1;
                println!("{}", "panic".red());
            }
            self.current_test += 1;
        }
        (passed, failed, self.filtered.len())
    }

    fn skip_filtered(&mut self) {
        self.filtered.push(self.current_test);
    }

    fn register_test(
        &mut self,
        name: impl Into<String>,
        test: impl Fn() + std::panic::UnwindSafe + 'static,
    ) {
        self.tests.push((name.into(), Box::new(test)));
    }
}

static mut TEST_RUNNER: TestRunner = TestRunner {
    tests: vec![],
    filtered: vec![],
    current_test: 0,
};

#[macro_export]
macro_rules! skip {
    () => {
        unsafe { $crate::TEST_RUNNER.skip_filtered() };
    };
    ($string:expr) => {
        use colored::*;
        #[allow(static_mut_refs)]
        unsafe {
            $crate::TEST_RUNNER.skip_filtered()
        };
        print!("{} ", ("filtered: ".to_string() + $string).yellow());
    };
}

#[macro_export]
macro_rules! register_tests {
    ( $($fn_path:path$(,)?)* ) => {{
         $(
            #[allow(static_mut_refs)]
            unsafe { $crate::TEST_RUNNER.register_test(stringify!($fn_path),$fn_path) }
         )*
    }};
}

#[allow(static_mut_refs)]
fn main() {
    register_test();
    let to_pass = unsafe { TEST_RUNNER.len() };
    println!("running {} tests", to_pass);
    let start = std::time::Instant::now();
    let result = unsafe { TEST_RUNNER.run() };
    let duration = start.elapsed();
    let ok = result.1 < 1;
    println!(
        "\ntest result: {}. {} passed; {} failed; {} filtered out; finished in {:.2}s\n",
        if ok { "ok".green() } else { "FAILED".red() },
        result.0,
        result.1,
        result.2,
        duration.as_secs_f64()
    );
    if !ok {
        std::process::exit(-1)
    }
}
