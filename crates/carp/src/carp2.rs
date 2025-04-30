use crate::model::CarpGraph;
use serde::{Serialize, Deserialize};

/// Starting at the beginning of the file, the header details specific information
/// about the file.
///
/// 1. `CG` tag (2 bytes)
/// 2. version number (2 bytes)
/// 3. width of graph (4 bytes)
/// 4. height of graph (4 bytes)
/// 5. `END_OF_HEADER`
///
/// The header has a total of 13 bytes. (12 of info, 1 of `END_OF_HEADER)
///
/// Everything after `END_OF_HEADER` should be another command and its parameters.
pub const END_OF_HEADER: u8 = 0x1a;
/// The color command marks the beginning of a hex-encoded color **string**.
///
/// The hastag character should **not** be included.
pub const COLOR: u8 = 0x1b;
/// The size command marks the beginning of a integer brush size.
pub const SIZE: u8 = 0x2b;
/// Marks the beginning of a new line.
pub const LINE: u8 = 0x3b;
/// A point marks the coordinates (relative to the previous `DELTA_ORIGIN`, or `(0, 0)`)
/// in which a point should be drawn.
///
/// The size and color are that of the previous `COLOR` and `SIZE` commands.
///
/// Points are two `u32`s (or 8 bytes in length).
pub const POINT: u8 = 0x4b;
/// An end-of-file marker.
pub const EOF: u8 = 0x1f;

/// A type of [`Command`].
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum CommandType {
    /// [`END_OF_HEADER`]
    EndOfHeader = END_OF_HEADER,
    /// [`COLOR`]
    Color = COLOR,
    /// [`SIZE`]
    Size = SIZE,
    /// [`LINE`]
    Line = LINE,
    /// [`POINT`]
    Point = POINT,
    /// [`EOF`]
    Eof = EOF,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Command {
    /// The type of the command.
    pub r#type: CommandType,
    /// Raw data as bytes.
    pub data: Vec<u8>,
}

impl From<Command> for Vec<u8> {
    fn from(val: Command) -> Self {
        let mut d = val.data;
        d.insert(0, val.r#type as u8);
        d
    }
}

/// A graph is CarpGraph's representation of an image. It's essentially just a
/// reproducable series of commands which a renderer can traverse to reconstruct
/// an image.
#[derive(Serialize, Deserialize, Debug)]
pub struct Graph {
    pub header: Vec<u8>,
    pub dimensions: (u32, u32),
    pub commands: Vec<Command>,
}

macro_rules! select_bytes {
    ($count:literal, $from:ident) => {{
        let mut data: Vec<u8> = Vec::new();
        let mut seen_bytes = 0;

        while let Some((_, byte)) = $from.next() {
            seen_bytes += 1;
            data.push(byte.to_owned());

            if seen_bytes == $count {
                // we only need <count> bytes, stop just before we eat the next byte
                break;
            }
        }

        data
    }};
}

macro_rules! spread {
    ($into:ident, $from:expr) => {
        for byte in &$from {
            $into.push(byte.to_owned())
        }
    };
}

impl CarpGraph for Graph {
    fn to_bytes(&self) -> Vec<u8> {
        let mut out: Vec<u8> = Vec::new();

        // reconstruct header
        spread!(out, self.header);
        spread!(out, self.dimensions.0.to_be_bytes()); // width
        spread!(out, self.dimensions.1.to_be_bytes()); // height
        out.push(END_OF_HEADER);

        // reconstruct commands
        for command in &self.commands {
            out.push(command.r#type as u8);
            spread!(out, command.data);
        }

        // ...
        out.push(EOF);
        out
    }

    fn from_bytes(bytes: Vec<u8>) -> Self {
        let mut header: Vec<u8> = Vec::new();
        let mut dimensions: (u32, u32) = (0, 0);
        let mut commands: Vec<Command> = Vec::new();

        let mut in_header: bool = true;
        let mut byte_buffer: Vec<u8> = Vec::new(); // storage for bytes which need to construct a bigger type (like `u32`)

        let mut bytes_iter = bytes.iter().enumerate();
        while let Some((i, byte)) = bytes_iter.next() {
            let byte = byte.to_owned();
            match byte {
                END_OF_HEADER => in_header = false,
                COLOR => {
                    let data = select_bytes!(6, bytes_iter);
                    commands.push(Command {
                        r#type: CommandType::Color,
                        data,
                    });
                }
                SIZE => {
                    let data = select_bytes!(2, bytes_iter);
                    commands.push(Command {
                        r#type: CommandType::Size,
                        data,
                    });
                }
                POINT => {
                    let data = select_bytes!(8, bytes_iter);
                    commands.push(Command {
                        r#type: CommandType::Point,
                        data,
                    });
                }
                LINE => commands.push(Command {
                    r#type: CommandType::Line,
                    data: Vec::new(),
                }),
                EOF => break,
                _ => {
                    if in_header {
                        if (0..2).contains(&i) {
                            // tag
                            header.push(byte);
                        } else if (2..4).contains(&i) {
                            // version
                            header.push(byte);
                        } else if (4..8).contains(&i) {
                            // width
                            byte_buffer.push(byte);

                            if i == 7 {
                                // end, construct from byte buffer
                                let (bytes, _) = byte_buffer.split_at(size_of::<u32>());
                                dimensions.0 = u32::from_be_bytes(bytes.try_into().unwrap());
                                byte_buffer = Vec::new();
                            }
                        } else if (8..12).contains(&i) {
                            // height
                            byte_buffer.push(byte);

                            if i == 11 {
                                // end, construct from byte buffer
                                let (bytes, _) = byte_buffer.split_at(size_of::<u32>());
                                dimensions.1 = u32::from_be_bytes(bytes.try_into().unwrap());
                                byte_buffer = Vec::new();
                            }
                        }
                    } else {
                        // misc byte
                        println!("extraneous byte at {i}");
                    }
                }
            }
        }

        Self {
            header,
            dimensions,
            commands,
        }
    }

    fn to_svg(&self) -> String {
        let mut out: String = String::new();
        out.push_str(&format!(
            "<svg viewBox=\"0 0 {} {}\" xmlns=\"http://www.w3.org/2000/svg\" style=\"background: white; width: {}px; height: {}px\" class=\"carpgraph\">",
            self.dimensions.0, self.dimensions.1, self.dimensions.0, self.dimensions.1
        ));

        // add lines
        let mut stroke_size: u16 = 2;
        let mut stroke_color: String = "000000".to_string();

        let mut previous_x_y: Option<(u32, u32)> = None;
        let mut line_path = String::new();

        for command in &self.commands {
            match command.r#type {
                CommandType::Size => {
                    let (bytes, _) = command.data.split_at(size_of::<u16>());
                    stroke_size = u16::from_be_bytes(bytes.try_into().unwrap_or([0, 0]));
                }
                CommandType::Color => {
                    stroke_color =
                        String::from_utf8(command.data.to_owned()).unwrap_or("#000000".to_string())
                }
                CommandType::Line => {
                    if !line_path.is_empty() {
                        out.push_str(&format!(
                            "<path d=\"{line_path}\" stroke=\"#{stroke_color}\" stroke-width=\"{stroke_size}\" />"
                        ));
                    }

                    previous_x_y = None;
                    line_path = String::new();
                }
                CommandType::Point => {
                    let (x, y) = command.data.split_at(size_of::<u32>());
                    let point = ({ u32::from_be_bytes(x.try_into().unwrap()) }, {
                        u32::from_be_bytes(y.try_into().unwrap())
                    });

                    // add to path string
                    line_path.push_str(&format!(
                        " M{} {}{}",
                        point.0,
                        point.1,
                        if let Some(pxy) = previous_x_y {
                            // line to there
                            format!(" L{} {}", pxy.0, pxy.1)
                        } else {
                            String::new()
                        }
                    ));

                    previous_x_y = Some((point.0, point.1));

                    // add circular point
                    out.push_str(&format!(
                        "<circle cx=\"{}\" cy=\"{}\" r=\"{}\" fill=\"#{stroke_color}\" />",
                        point.0,
                        point.1,
                        stroke_size / 2 // the size is technically the diameter of the circle
                    ));
                }
                _ => unreachable!("never pushed to commands"),
            }
        }

        if !line_path.is_empty() {
            out.push_str(&format!(
                "<path d=\"{line_path}\" stroke=\"#{stroke_color}\" stroke-width=\"{stroke_size}\" />"
            ));
        }

        // return
        format!("{out}</svg>")
    }
}
