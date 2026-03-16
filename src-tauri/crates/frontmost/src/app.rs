use std::fmt::Debug;

pub trait FrontmostApp: Debug {
    fn set_frontmost(&mut self, new_value: &str);
    fn update(&mut self);
}
