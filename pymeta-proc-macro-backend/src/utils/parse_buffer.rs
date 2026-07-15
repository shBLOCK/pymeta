use crate::utils::ResultOrOption;
use std::ops::RangeBounds;
use std::rc::Rc;
use std::slice::SliceIndex;

pub(crate) struct ParseBuffer<T> {
    items: Rc<[T]>,
    pos: usize,
}

impl<T> ParseBuffer<T> {
    pub fn read_one(&mut self) -> Option<&T> {
        self.current()?;
        self.pos += 1;
        Some(self.peek(-1).unwrap())
    }

    pub fn seek(&mut self, offset: isize) -> Option<&mut Self> {
        let target = self.pos as isize + offset;
        if !(0..=self.items.len() as isize).contains(&target) {
            return None;
        }
        self.pos = target as usize;
        Some(self)
    }

    pub fn seeked(&self, offset: isize) -> Option<Self> {
        let mut buf = self.clone();
        buf.seek(offset)?;
        Some(buf)
    }

    pub fn current(&self) -> Option<&T> {
        self.items.get(self.pos)
    }

    pub fn slice(&self, range: impl RangeBounds<usize> + SliceIndex<[T], Output = [T]>) -> &[T] {
        &self.items[range]
    }

    pub fn peek(&self, offset: isize) -> Option<&T> {
        self.items.get(self.pos.checked_add_signed(offset)?)
    }

    pub fn pos(&self) -> usize {
        self.pos
    }

    #[allow(unused)]
    pub fn set_pos(&mut self, pos: usize) -> Result<(), ()> {
        if pos > self.items.len() {
            return Err(());
        }
        self.pos = pos;
        Ok(())
    }

    pub fn exhausted(&self) -> bool {
        self.pos >= self.items.len()
    }

    pub fn try_run_or_rewind<R, E, RE: ResultOrOption<R, E>>(&mut self, f: impl FnOnce(&mut Self) -> RE) -> RE {
        let pos = self.pos;
        let result = f(self);
        if result.is_bad() {
            self.pos = pos;
        }
        result
    }
}

impl<T> Clone for ParseBuffer<T> {
    fn clone(&self) -> Self {
        Self {
            items: self.items.clone(),
            pos: self.pos,
        }
    }
}

impl<T> From<Rc<[T]>> for ParseBuffer<T> {
    fn from(value: Rc<[T]>) -> Self {
        Self { items: value, pos: 0 }
    }
}

impl<T> FromIterator<T> for ParseBuffer<T> {
    fn from_iter<Iter: IntoIterator<Item = T>>(iter: Iter) -> Self {
        Self::from(Rc::from_iter(iter))
    }
}
