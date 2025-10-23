use std::{
    fs::File,
    io::{BufReader, Read},
    os::unix::fs::MetadataExt,
};

// byte-aligned suffix (01100000)
const SHA_SUFFIX: u8 = 96;

// centered array map
const CAM: [usize; 5] = [2, 3, 4, 0, 1];

const RHO_TABLE: [[u64; 5]; 5] = [
    [21, 120, 28, 55, 153],
    [136, 78, 91, 276, 231],
    [105, 210, 0, 36, 3],
    [45, 66, 1, 300, 10],
    [15, 253, 190, 6, 171],
];

fn static_reverse_u64_bits(number: u64) -> u64 {
    let mut bytes = number.to_be_bytes();
    for byte in bytes.iter_mut() {
        let mut b = *byte;
        let mut reversed = 0;
        for _ in 0..8 {
            reversed = (reversed << 1) | (b & 1);
            b >>= 1;
        }
        *byte = reversed;
    }

    return u64::from_be_bytes(bytes);
}

const IOTA_TABLE: [u64; 24] = [
    9223372036854775808,
    4684025087442026496,
    5836946592048873473,
    281479271677953,
    15060318628903649280,
    9223372041149743104,
    9295711110164381697,
    10376575016438333441,
    5836665117072162816,
    1224979098644774912,
    10376575020733300736,
    5764607527329202176,
    15060318633198616576,
    15060037153926938625,
    10448632610476261377,
    13835339530258874369,
    4611967493404098561,
    72057594037927937,
    5764888998010945536,
    5764607527329202177,
    9295711110164381697,
    72339069014638593,
    9223372041149743104,
    1153202983878524929,
];

#[derive(Debug, Clone, Copy)]
pub enum Mode {
    Sha3_224(usize),
    Sha3_256(usize),
    Sha3_384(usize),
    Sha3_512(usize),
}

impl TryFrom<&String> for Mode {
    type Error = &'static str;

    fn try_from(value: &String) -> Result<Self, Self::Error> {
        match value.to_lowercase().as_str() {
            "224" => Ok(Self::Sha3_224(144)),
            "256" => Ok(Self::Sha3_256(136)),
            "384" => Ok(Self::Sha3_384(104)),
            "512" => Ok(Self::Sha3_512(72)),
            _ => Err("Invalide mode selected"),
        }
    }
}

impl Default for Mode {
    fn default() -> Self {
        Mode::Sha3_224(144)
    }
}

pub struct Sponge {
    mode: Mode,
    state: [[u64; 5]; 5],
}

impl Sponge {
    pub fn new(mode: Mode) -> Self {
        return Sponge {
            mode,
            state: [[0; 5]; 5],
        };
    }

    fn reverse_bits_in_place(byte_slice: &mut [u8]) {
        for byte in byte_slice.iter_mut() {
            let mut b = *byte;
            let mut reversed = 0;
            for _ in 0..8 {
                reversed = (reversed << 1) | (b & 1);
                b >>= 1;
            }
            *byte = reversed;
        }
    }

    fn theta(&mut self) {
        let mut c: [u64; 5] = [0; 5];
        for x in 0..=4 {
            c[x] = self.state[x][0]
                ^ self.state[x][1]
                ^ self.state[x][2]
                ^ self.state[x][3]
                ^ self.state[x][4];
        }

        let mut d: [u64; 5] = [0; 5];
        for x in 0..=4 {
            d[x] = c[(x + 4) % 5] ^ (c[(x + 1) % 5].rotate_right(1));
            for y in 0..=4 {
                self.state[x][y] = self.state[x][y] ^ d[x];
            }
        }
    }

    fn rho(&mut self) {
        for x in 0..=4 {
            for y in 0..=4 {
                self.state[x][y] = self.state[x][y].rotate_right(RHO_TABLE[CAM[x]][CAM[y]] as u32);
            }
        }
    }

    fn pi(&mut self) {
        let mut new_state: [[u64; 5]; 5] = [[0; 5]; 5];

        for x in 0..=4 {
            for y in 0..=4 {
                new_state[x][y] = self.state[(x + (3 * y)) % 5][x]
            }
        }

        self.state = new_state;
    }

    fn chi(&mut self) {
        let mut new_state: [[u64; 5]; 5] = [[0; 5]; 5];

        for x in 0..=4 {
            for y in 0..=4 {
                new_state[x][y] = self.state[x][y]
                    ^ ((!(self.state[(x + 1) % 5][y])) & self.state[(x + 2) % 5][y]);
            }
        }

        self.state = new_state;
    }

    fn iota(&mut self, round: usize) {
        self.state[0][0] = self.state[0][0] ^ IOTA_TABLE[round];
    }

    pub fn absorb(&mut self, file_path: &String) {
        let file_meta = std::fs::metadata(file_path).unwrap();
        let file_size: usize = file_meta.size().try_into().unwrap();

        let file_handle = File::open(file_path).unwrap();
        let mut file_reader = BufReader::new(file_handle);

        match self.mode {
            Mode::Sha3_224(bit_rate)
            | Mode::Sha3_256(bit_rate)
            | Mode::Sha3_384(bit_rate)
            | Mode::Sha3_512(bit_rate) => {
                let mut break_flag = false;

                while !break_flag {
                    let mut buffer = vec![0; bit_rate.try_into().unwrap()];
                    let read_result = file_reader.read_exact(&mut buffer);
                    Sponge::reverse_bits_in_place(&mut buffer);

                    match read_result {
                        Err(error) => match error.kind() {
                            std::io::ErrorKind::UnexpectedEof => {
                                let padding_start_index: usize =
                                    (file_size % bit_rate).try_into().unwrap();

                                if padding_start_index == bit_rate - 1 {
                                    buffer[padding_start_index] = SHA_SUFFIX + 1;
                                } else {
                                    buffer[padding_start_index] = SHA_SUFFIX;
                                    buffer[bit_rate - 1] = 1;
                                }

                                break_flag = true;
                            }
                            _ => {}
                        },
                        _ => {}
                    }

                    for lane in 0..(bit_rate / 8) {
                        let x = lane % 5;
                        let y = lane / 5;
                        let slice = &buffer[(lane * 8)..((lane * 8) + 8)];

                        self.state[x][y] =
                            self.state[x][y] ^ u64::from_be_bytes(slice.try_into().unwrap());
                    }

                    for round in 0..=23 {
                        self.theta();
                        self.rho();
                        self.pi();
                        self.chi();
                        self.iota(round);
                    }
                }
            }
        }
    }

    pub fn squeeze(&mut self) -> String {
        match self.mode {
            Mode::Sha3_224(bit_rate)
            | Mode::Sha3_256(bit_rate)
            | Mode::Sha3_384(bit_rate)
            | Mode::Sha3_512(bit_rate) => {
                for x in 0..=4 {
                    for y in 0..=4 {
                        self.state[x][y] = static_reverse_u64_bits(self.state[x][y]);
                    }
                }

                let mut output_hex_vec = vec![];
                for lane in 0..(bit_rate / 8) {
                    output_hex_vec.push(format!("{:016x}", self.state[lane % 5][lane / 5]));
                }

                let mut output_hex = output_hex_vec.join("");

                match self.mode {
                    Mode::Sha3_224(_) => {
                        output_hex.truncate(224 / 4);
                    }
                    Mode::Sha3_256(_) => {
                        output_hex.truncate(256 / 4);
                    }
                    Mode::Sha3_384(_) => {
                        output_hex.truncate(384 / 4);
                    }
                    Mode::Sha3_512(_) => {
                        output_hex.truncate(512 / 4);
                    }
                }

                return output_hex;
            }
        }
    }
}

pub fn run_test() {
    todo!();
}
