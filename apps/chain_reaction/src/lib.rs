use std::cmp::min;
use std::error::Error;
use std::time::Duration;
use std::time::Instant;

use midichan_core::message::{MidiMessage, MessageType};
use midichan_core::device::Application;
use launchpad::{Launchpad, Color};

#[derive(Clone, Copy, Debug, Default)]
pub struct Field(pub u8);

impl Field {
    #[inline]
    pub fn count(&self) -> u8 {
        self.0 & 0xF
    }

    #[inline]
    pub fn set_count(&mut self, count: u8) {
        self.0 = self.0 & !0xF | (count & 0xF);
        if count == 0 { self.set_player(0); }
    }

    #[inline]
    pub fn add_count(&mut self, count: u8) {
        self.set_count(self.count() + count);
    }

    #[inline]
    pub fn sub_count(&mut self, count: u8) {
        self.set_count(self.count() - count);
    }

    #[inline]
    pub fn player(&self) -> u8 {
        (self.0 & 0x70) >> 4
    }

    #[inline]
    pub fn set_player(&mut self, player: u8) {
        self.0 = self.0 & !0x70 | (player << 4);
    }

    #[inline]
    pub fn boom(&self) -> bool {
        self.0 & 0x80 == 0x80
    }

    #[inline]
    pub fn set_boom(&mut self, boom: bool) {
        self.0 = self.0 & !0x80 | (boom as u8 * 0x80);
    }
}

type Board = [[Field; 8]; 8];

pub struct ChainReaction {
    board: Board,
    colors: Vec<Vec<Color>>,
    next_player: u8,
    player_count: u8,
    launchpad: Launchpad
}

fn midi_to_item(msg: &MidiMessage) -> Option<(u8, u8)> {
    let row = msg.key / 16;
    let col = msg.key % 16;
    if col >= 8 || msg.msg_type == MessageType::CC {
        None
    } else {
        Some((row, col))
    }
}

impl ChainReaction {
    pub fn new(launchpad: Launchpad) -> ChainReaction {
        ChainReaction{
            board: Default::default(),
            colors: vec![
                vec![Color::new(0, 0)],
                vec![Color::new(0, 0), Color::new(1, 0), Color::new(3, 0),Color::new(3, 1)],
                vec![Color::new(0, 0), Color::new(0, 1), Color::new(0, 3), Color::new(1, 3)],
                vec![Color::new(0, 0), Color::new(1, 1), Color::new(2, 2), Color::new(3, 3)]
            ],
            next_player: 0,
            player_count: 2,
            launchpad: launchpad
        }
    }

    fn reset(&mut self) -> Result<(), Box<dyn Error>> {

        for row in 0..7 {
            for col in 0..7 {
                self.board[row][col] = Field::default();
                self.render(row, col)?;
            }
        }

        
        Ok(())
    }

    fn render(&self, row: usize, col: usize) -> Result<(), Box<dyn Error>> {
        let item = self.board[row][col];
        self.launchpad.set(col as u8, row as u8, &self.colors[item.player() as usize][min(item.count() as usize, 3)])
    }

    fn tick(&mut self) -> Result<(), Box<dyn Error>> {
        let mut clone = self.board.clone();

        for row in 0..7 {
            for col in 0..7 {
                clone[row][col].set_boom(false);
            }
        }

        for row in 0..7 {
            for col in 0..7 {
                let explosion = match (row % 7, col % 7) {
                    (0, 0) => 2,
                    (0, _) | (_, 0) => 3,
                    _ => 4
                };

                let item = self.board[row][col];
                
                if item.count() > explosion {
                    clone[row][col].sub_count(explosion);

                    self.render(row, col)?;

                    for &(x, y) in &[(row-1, col-1), (row-1, col+1), (row+1, col-1), (row+1, col+1)] {
                        clone.get_mut(x)
                            .and_then(|i| i.get_mut(y))
                            .map(|i| {
                                i.add_count(1);
                                i.set_player(item.player());
                                i.set_boom(true);
                            });

                        self.render(x, y)?;
                    }
                }
            }
        }

        self.board = clone;

        Ok(())
    }

    fn step(&mut self, row: u8, col: u8) -> bool {
        let item = &mut self.board[row as usize][col as usize];

        self.next_player += 1;

        let change = {
            let player = item.player();
            player == self.next_player || player == 0
        };

        if change {
            item.add_count(1);
            item.set_player(self.next_player);
            self.next_player %= self.player_count;
        }

        change
    }
}

impl Application for ChainReaction {
    fn run(&mut self) -> Result<(), Box<dyn Error>> {
        let midi_in = self.launchpad.input();

        let mut time = Instant::now();

        self.reset()?;

        loop {
            match midi_in.recv_timeout(Duration::from_millis(50)) {
                Ok(MidiMessage { msg_type: MessageType::CC, key: 111, velocity: vel, .. }) => {
                    if vel == 127 {
                        self.launchpad.clear()?;
                        break;
                    }
                },

                Ok(msg) => {
                    if let Some((row, col)) = midi_to_item(&msg) {
                        if msg.velocity == 127 {
                            self.step(row, col);
                            self.launchpad.set(col, row, &Color::new(3, 3))?;
                        } else {
                            self.render(row as usize, col as usize)?;
                        }
                    }
                }
                _ => {
                    continue;
                }
            }

            let new_time = Instant::now();

            if new_time - time > Duration::from_millis(800) {
                time = new_time;
                self.tick()?;
            }
        }

        Ok(())
    }
}