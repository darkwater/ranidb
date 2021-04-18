use std::fmt::{Display, Write};

pub(crate) struct CommandBuilder {
    inner: String,
    add_amp: bool,
}

impl CommandBuilder {
    pub fn new(name: &str) -> Self {
        Self {
            inner: format!("{} ", name),
            add_amp: false,
        }
    }

    pub fn arg<T: CmdArgument>(mut self, key: &str, value: T) -> Self {
        if self.add_amp {
            self.inner.push('&');
        }
        else {
            self.add_amp = true;
        }

        self.inner.push_str(&format!("{}={}", key, value.escaped()));

        self
    }

    pub fn build(mut self) -> String {
        self.inner.push('\n');
        self.inner
    }
}

impl Display for CommandBuilder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.inner)
    }
}

pub(crate) trait CmdArgument {
    type Output: Display;

    fn escaped(&self) -> Self::Output;
}

macro_rules! simple_cmdarg {
    ($($t:ty)*) => {
        $(
            impl CmdArgument for $t {
                type Output = $t;

                fn escaped(&self) -> Self::Output {
                    *self
                }
            }
        )*
    };
}

simple_cmdarg!(i8 i16 i32 i64 isize u8 u16 u32 u64 usize);

impl CmdArgument for &str {
    type Output = String;

    fn escaped(&self) -> String {
        self.replace('&', "&amp;").replace('\n', "<br />")
    }
}

impl CmdArgument for &[u8] {
    type Output = String;

    fn escaped(&self) -> String {
        let mut out = String::new();

        for byte in self.iter() {
            write!(out, "{:02x}", byte).unwrap();
        }

        out
    }
}

impl CmdArgument for bool {
    type Output = u8;

    fn escaped(&self) -> Self::Output {
        if *self {
            1
        }
        else {
            0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        assert_eq!(
            CommandBuilder::new("FOO")
                .arg("user", "name")
                .arg("pass", "w&rd")
                .arg("weeb", true)
                .arg("iq", 9000)
                .arg("bytes", &[0xd0, 0x0d][..])
                .build(),
            "FOO user=name&pass=w&amp;rd&weeb=1&iq=9000&bytes=d00d\n"
        );
    }
}
