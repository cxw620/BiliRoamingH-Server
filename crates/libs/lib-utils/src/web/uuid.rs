// Copyright (c) 2024 Hantong Chen(cxw620). MIT license.
// RE from `https://s1.hdslb.com/bfs/seed/log/report/log-reporter.js`
// Last Updated: 2024/1/12 16:33, js content md5: a6fa378028e0cce7ea7202dda4783781

use crate::{now, random_choice, str_concat};

#[allow(dead_code)]
struct UuidInfoc;

#[allow(dead_code)]
impl UuidInfoc {
    pub fn gen() -> String {
        // Math.random() === 0 is really rare that probability is unlimited close to 0
        static DIGHT_MAP: [&'static str; 16] = [
            "1", "2", "3", "4", "5", "6", "7", "8", "9", "A", "B", "C", "D", "E", "F", "10",
        ];
        let t = now!().as_millis() % 100_000;

        str_concat!(
            &random_choice!(8, 4, 4, 4, 12; "-"; DIGHT_MAP),
            &format!("{:0>5}", t),
            "infoc"
        )
    }
}
