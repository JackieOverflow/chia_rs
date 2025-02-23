use clvmr::allocator::Allocator;
use clvmr::allocator::NodePtr;

pub struct BitCursor<'a> {
    data: &'a [u8],
    bit_offset: u8,
}

fn mask(num: u8) -> u8 {
    0xff >> num
}

impl<'a> BitCursor<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        BitCursor {
            data,
            bit_offset: 0,
        }
    }

    pub fn read_bits(&mut self, mut num: u8) -> Option<u8> {
        assert!(num <= 8);
        let ret = if self.data.is_empty() {
            num = 0;
            None
        } else if self.bit_offset + num <= 8 {
            Some((self.data[0] & mask(self.bit_offset)) >> (8 - num - self.bit_offset))
        } else if self.data.len() < 2 {
            num = 8 - self.bit_offset;
            Some(self.data[0] & mask(self.bit_offset))
        } else {
            let first_byte = 8 - self.bit_offset;
            let second_byte = num - first_byte;
            Some(
                ((self.data[0] & mask(self.bit_offset)) << second_byte)
                    | (self.data[1] >> (8 - second_byte)),
            )
        };
        self.advance(num);
        ret
    }

    fn advance(&mut self, bits: u8) {
        let bits = self.bit_offset as u32 + bits as u32;
        if bits >= 8 {
            self.data = &self.data[(bits / 8) as usize..];
        }
        self.bit_offset = (bits % 8) as u8;
    }
}

const BUFFER: [u8; 63] = [
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
];

enum MakeTreeOp {
    Pair,
    Tree,
}

pub fn make_tree(a: &mut Allocator, cursor: &mut BitCursor, short_atoms: bool) -> NodePtr {
    let mut value_stack = Vec::<NodePtr>::new();
    let mut op_stack = vec![MakeTreeOp::Tree];

    while !op_stack.is_empty() {
        match op_stack.pop().unwrap() {
            MakeTreeOp::Pair => {
                let second = value_stack.pop().unwrap();
                let first = value_stack.pop().unwrap();
                value_stack.push(a.new_pair(first, second).unwrap());
            }
            MakeTreeOp::Tree => match cursor.read_bits(1) {
                None => value_stack.push(a.nil()),
                Some(0) => {
                    op_stack.push(MakeTreeOp::Pair);
                    op_stack.push(MakeTreeOp::Tree);
                    op_stack.push(MakeTreeOp::Tree);
                }
                Some(_) => {
                    let atom = if short_atoms {
                        match cursor.read_bits(8) {
                            None => a.nil(),
                            Some(val) => a.new_atom(&[val]).unwrap(),
                        }
                    } else {
                        match cursor.read_bits(6) {
                            None => a.nil(),
                            Some(len) => a.new_atom(&BUFFER[..len as usize]).unwrap(),
                        }
                    };
                    value_stack.push(atom);
                }
            },
        }
    }

    assert!(value_stack.len() == 1);
    value_stack.pop().unwrap()
}
