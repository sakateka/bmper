use bmp;

/// http://www.binaryessence.com/dct/en000073.htm
/// Start marker
pub const RLE_MARK: u8 = 0x00;
/// The end of line marker indicates that the next code will apply to a new line
pub const RLE_EOL: u8 = 0x00;
/// The end of bitmap marker indicates the end of encoded data
pub const RLE_EOB: u8 = 0x01;
/// The delta marker indicates a jump relative to the current position.
pub const RLE_DELTA: u8 = 0x02;

pub trait Rle8 {
    fn encode(&mut self, _width: i32, _height: i32);
    fn decode(&mut self, width: i32, height: i32);
}
pub trait Rle4 {
    fn encode(&mut self, _width: i32, _height: i32);
    fn decode(&mut self, _width: i32, _height: i32);
}

impl Rle8 for bmp::Bitmap {
    fn encode(&mut self, _width: i32, _height: i32) {
        match self.decoded {
            None => return,
            _ => (),
        }
        unimplemented!();
        //self.decoded = Some(bmp::BMPCompression::RLE8);
        //self.data = Vec::new();
    }
    fn decode(&mut self, width: i32, height: i32) {
        assert!(height > 0);
        match self.decoded {
            None => (),
            _ => return,
        }
        let mut decoded_bm = Vec::new();
        let mut x: i32 = 0;
        let mut y = height - 1;
        {
            let mut it = self.data.iter();
            loop {
                let first;
                let second;
                match it.next() {
                    Some(val) => first = *val,
                    None => break,
                };
                match it.next() {
                    Some(val) => second = *val,
                    None => break,
                };

                match first {
                    RLE_MARK => {
                        match second {
                            RLE_EOB => break,
                            RLE_EOL => {
                                let line_pad = width - x;
                                if x > 0 && line_pad > 0 {
                                    let len = decoded_bm.len();
                                    decoded_bm.resize(len + line_pad as usize, 0u8);
                                }
                                x = 0;
                                y -= 1;
                            },
                            RLE_DELTA => {
                                let delta_x;
                                let delta_y;
                                match it.next() {
                                    Some(val) => delta_x = *val as i32,
                                    None => break,
                                };
                                match it.next() {
                                    Some(val) => delta_y = *val as i32,
                                    None => break,
                                };
                                let len = decoded_bm.len();
                                let append = (delta_x + delta_y * width) as usize;
                                x += delta_x;
                                y -= delta_y;
                                decoded_bm.resize(len + append, 0u8);
                            },
                            _ => { // absolute mode
                                let with_word_pad = ((second + 1) / 2) * 2;
                                for _ in 0..with_word_pad {
                                    match it.next() {
                                        Some(val) => {
                                            decoded_bm.push(*val);
                                        }
                                        None => break,
                                    };
                                }
                                x += second as i32;
                            }
                        }
                    }
                    _ => { // encoded mode
                        let len = decoded_bm.len();
                        decoded_bm.resize(len + first as usize, second);
                        x += first as i32;
                        if x >= width {
                            x = x % width;
                            y -= 1;
                        }
                    }
                }
            }
        }
        let append = width - x + width * y;
        if append > 0 {
            let len = decoded_bm.len();
            decoded_bm.resize(len + append as usize, 0u8);
        }
        self.decoded = Some(bmp::BMPCompression::RLE8);
        self.data = decoded_bm;
    }
}

impl Rle4 for bmp::Bitmap {
    fn encode(&mut self, _width: i32, _height: i32) {
        unimplemented!();
    }
    fn decode(&mut self, _width: i32, _height: i32) {
        unimplemented!();
    }
}

/*
fn encode_bitfields(&mut self) {
    unimplemented!()
}

fn encode_jpeg(&mut self) {
    unimplemented!()
}

fn encode_png(&mut self) {
    unimplemented!()
}
*/


/*
fn decode_bitfields(&mut self) {
    unimplemented!()
}

fn decode_jpeg(&mut self) {
    unimplemented!()
}

fn decode_png(&mut self) {
    unimplemented!()
}
*/
