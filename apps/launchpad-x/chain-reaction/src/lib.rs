use std::cmp::min;
use std::error::Error;
use std::time::Duration;
use std::time::Instant;

use midichan_core::message::{MidiMessage, MessageType};
use midichan_core::device::Application;
use launchpad_x::*;

#[derive(Clone, Copy, Debug, Default)]
pub struct Field(pub u8);

impl Field {
    #[inline]
    pub fn count(&self) -> u8 {
        self.0 & 0xF
    }

    #[inline]
    pub fn set_count(&mut self, count: u8) {
        self.0 = (self.0 & !0xF) | (count & 0xF);
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
        self.0 = (self.0 & !0x70) | (player << 4);
    }

    #[inline]
    pub fn boom(&self) -> bool {
        (self.0 & 0x80) == 0x80
    }

    #[inline]
    pub fn set_boom(&mut self, boom: bool) {
        self.0 = (self.0 & !0x80) | (boom as u8 * 0x80);
    }
}

type Board = [[Field; 8]; 8];

pub struct ChainReaction {
    board: Board,
    colors: Vec<Vec<Color>>,
    next_player: u8,
    player_count: u8,
    has_boom: bool,
    launchpad: LaunchpadX
}

fn midi_to_item(msg: &MidiMessage) -> Option<(u8, u8)> {
    let row = msg.key / 10;
    let col = msg.key % 10;
    if col > 8 || row == 0 || col == 0 || msg.msg_type == MessageType::CC {
        None
    } else {
        Some((row-1, col-1))
    }
}

impl ChainReaction {
    pub fn new(launchpad: LaunchpadX) -> ChainReaction {
        ChainReaction{
            board: Default::default(),
            colors: vec![
                vec![lpx_color!(0)],
                vec![lpx_color!(0), lpx_color!(7), lpx_color!(6), lpx_color!(5)],
                vec![lpx_color!(0), lpx_color!(43), lpx_color!(42), lpx_color!(41)],
                vec![lpx_color!(0), lpx_color!(27), lpx_color!(26), lpx_color!(25)],
                vec![lpx_color!(0), lpx_color!(15), lpx_color!(14), lpx_color!(13)],
                vec![lpx_color!(0), lpx_color!(55), lpx_color!(54), lpx_color!(53)],
            ],
            next_player: 1,
            player_count: 2,
            has_boom: false,
            launchpad
        }
    }

    fn reset(&mut self) -> Result<(), Box<dyn Error>> {
        for row in 0..=7 {
            for col in 0..=7 {
                self.board[row][col] = Field::default();
                self.render(row, col)?;
            }
        }

        self.next_player = 1;
        self.launchpad.set(8, 8, self.colors[self.next_player as usize][3])?;
        
        Ok(())
    }

    fn render(&self, row: usize, col: usize) -> Result<(), Box<dyn Error>> {
        let item = self.board[row][col];
        self.launchpad.set(col as u8, row as u8, self.colors[item.player() as usize][min(item.count() as usize, 3)])?;
        Ok(())
    }

    fn render_new(&self, row: usize, col: usize, field: Field) -> Result<(), Box<dyn Error>> {
        let item = field;
        self.launchpad.set(col as u8, row as u8, self.colors[item.player() as usize][min(item.count() as usize, 3)])?;
        Ok(())
    }

    fn tick(&mut self) -> Result<(), Box<dyn Error>> {
        let mut clone: Vec<[Field; 8]> = self.board.iter().cloned().collect();

        for row in 0..=7 {
            for col in 0..=7 {
                clone[row][col].set_boom(false);
            }
        }
        self.has_boom = false;

        for row in 0..=7 {
            for col in 0..=7 {
                let explosion = match (row % 7, col % 7) {
                    (0, 0) => 2,
                    (0, _) | (_, 0) => 3,
                    _ => 4
                };

                let item = self.board[row][col];
                
                if item.count() >= explosion {
                    let player = item.player();
                    clone[row][col].sub_count(explosion);

                    self.render_new(row, col, clone[row][col])?;

                    for &(x, y) in &[
                        (Some(row), col.checked_sub(1)),
                        (Some(row), col.checked_add(1)),
                        (row.checked_sub(1), Some(col)),
                        (row.checked_add(1), Some(col))
                    ] {
                        if let (Some(x), Some(y)) = (x, y) {
                            if let Some(a) = clone.get_mut(x) {
                                if let Some(i) = a.get_mut(y) {
                                    i.add_count(1);
                                    i.set_player(player);
                                    i.set_boom(true);
                                    self.has_boom = true;

                                    self.render_new(x, y, i.clone())?;
                                }
                            }
                        }
                    }
                }
            }
        }

        for (tgt, src) in self.board.iter_mut().zip(clone.iter()) {
            tgt.clone_from_slice(src);
        }

        Ok(())
    }

    fn step(&mut self, row: u8, col: u8) -> bool {
        let item = &mut self.board[row as usize][col as usize];


        let change = {
            let player = item.player();
            player == self.next_player || player == 0
        };

        if change {
            item.add_count(1);
            item.set_player(self.next_player);
            self.next_player %= self.player_count;
            self.next_player += 1;
        }

        change
    }
}

impl Application for ChainReaction {
    fn run(&mut self) -> Result<(), Box<dyn Error>> {
        let midi_in = self.launchpad.input();
        self.launchpad.set_programmer_mode(true)?;

        let mut time = Instant::now();

        self.reset()?;

        loop {
            match midi_in.recv_timeout(Duration::from_millis(50)) {
                Ok(MidiMessage { msg_type: MessageType::CC, key: 98, velocity: vel, .. }) => {
                    if vel > 0 {
                        self.launchpad.clear()?;
                        break;
                    }
                },

                Ok(msg) if matches!(msg.msg_type, MessageType::NoteOn) && !self.has_boom => {
                    if let Some((row, col)) = midi_to_item(&msg) {
                        if msg.velocity > 0 {
                            self.step(row, col);
                            self.tick()?;
                            self.launchpad.set(col, row, lpx_color!(36))?;
                            self.launchpad.set(8, 8, self.colors[self.next_player as usize][3])?;
                        } else {
                            self.render(row as usize, col as usize)?;
                        }
                    }
                }
                _ => ()
            }

            let new_time = Instant::now();

            if new_time - time > Duration::from_millis(800) {
                time = new_time;
                self.tick()?;
            }
        }

        self.launchpad.set_programmer_mode(false)?;
        Ok(())
    }
}