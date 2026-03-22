use alloc::string::{String, ToString};
use alloc::vec::Vec;

#[derive(Clone, Debug, Default)]
pub(crate) struct SliceCursor<'a> {
    pub(crate) buf: &'a [u8],
    pub(crate) pos: usize,
}

impl<'a> SliceCursor<'a> {
    pub(crate) fn new(buf: &'a [u8]) -> Self {
        Self { buf, pos: 0 }
    }

    pub(crate) fn position(&self) -> u64 {
        self.pos as u64
    }

    pub(crate) fn set_position(&mut self, pos: u64) {
        self.pos = pos as usize;
    }

    pub(crate) fn read_exact(&mut self, out: &mut [u8]) -> Result<(), String> {
        let remaining = self.buf.len().saturating_sub(self.pos);
        if out.len() > remaining {
            return Err("Unexpeted end of stream".to_string());
        }
        out.copy_from_slice(&self.buf[self.pos..self.pos + out.len()]);
        self.pos += out.len();
        Ok(())
    }

    pub(crate) fn read_to_end(&mut self, out: &mut Vec<u8>) -> Result<usize, String> {
        let remaining = self.buf.len().saturating_sub(self.pos);
        out.reserve(remaining);
        out.extend_from_slice(&self.buf[self.pos..]);
        self.pos = self.buf.len();
        Ok(remaining)
    }

    pub(crate) fn take(&mut self, n: u64) -> Self {
        Self {
            buf: &self.buf[self.pos..self.pos + n as usize],
            pos: 0,
        }
    }

    pub(crate) fn take_rest(&mut self) -> Self {
        Self {
            buf: &self.buf[self.pos..],
            pos: 0,
        }
    }
}
