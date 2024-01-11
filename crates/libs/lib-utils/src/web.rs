use crate::{now, random_choice, str_concat};

#[allow(dead_code)]
struct UuidInfoc;

#[allow(dead_code)]
impl UuidInfoc {
    pub fn gen() -> String {
        static DIGHT_MAP: [&'static str; 17] = [
            "0", "1", "2", "3", "4", "5", "6", "7", "8", "9", "A", "B", "C", "D", "E", "F", "10",
        ];
        let t = now!().as_millis() % 100_000;

        str_concat!(
            &random_choice!(8, 4, 4, 4, 12; "-"; DIGHT_MAP),
            &format!("{:0>5}", t),
            "infoc"
        )
    }
}
