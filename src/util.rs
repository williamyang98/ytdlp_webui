pub fn get_unix_time() -> u64 {
    use std::time::SystemTime;
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("System time before Unix epoch")
        .as_secs()
}

pub fn defer<F: FnOnce()>(f: F) -> impl Drop {
    use core::mem::ManuallyDrop;
    struct Defer<F: FnOnce()>(ManuallyDrop<F>);
    impl<F: FnOnce()> Drop for Defer<F> {
        fn drop(&mut self) {
            let f: F = unsafe { ManuallyDrop::take(&mut self.0) };
            f();
        }
    }
    Defer(ManuallyDrop::new(f))
}

#[macro_export]
macro_rules! generate_bidirectional_binding {
    ($type_enum:ty, $type_into:ty, $type_from:ty, $(($val_enum:ident, $val_bind:expr),)+) => {
        impl Into<$type_into> for $type_enum {
            fn into(self) -> $type_into {
                match self {
                    $(<$type_enum>::$val_enum => $val_bind,)+
                }
            }
        }

        impl TryFrom<$type_from> for $type_enum {
            type Error = &'static str;
            fn try_from(v: $type_from) -> Result<Self, Self::Error> {
                match v {
                    $($val_bind => Ok(<$type_enum>::$val_enum),)+
                    _ => Err("Invalid value to convert from"),
                }
            }
        }
    }
}

pub struct ConvertCarriageReturnToNewLine<T: std::io::Read> {
    reader: T,
}

impl<T: std::io::Read> ConvertCarriageReturnToNewLine<T> {
    pub fn new(reader: T) -> Self {
        Self { reader }
    }
}

impl<T: std::io::Read> std::io::Read for ConvertCarriageReturnToNewLine<T> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let res = self.reader.read(buf);
        if let Ok(total) = res {
            buf[..total].iter_mut().filter(|v| **v == b'\r').for_each(|v| *v = b'\n');
        }
        res
    }
}
