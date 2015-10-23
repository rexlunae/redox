use alloc::boxed::Box;

use common::random;
use schemes::{Resource, URL, VecResource};
use common::string::{String, ToString};

use schemes::KScheme;

/// A pseudorandomness scheme
pub struct RandomScheme;

impl KScheme for RandomScheme {
    fn scheme(&self) -> String {
        return "random".to_string();
    }

    fn open(&mut self, url: &URL) -> Option<Box<Resource>> {
        Some(box VecResource::new(URL::from_str("random://"), String::from_num(random::rand()).to_utf8()))
    }
}