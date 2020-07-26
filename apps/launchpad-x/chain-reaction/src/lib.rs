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

pub enum ChainState {
    Empty,
    Starting(u8),
    InProgress(u8),
    GameOver(u8)
}

pub struct ChainReaction {
    board: Board,
    colors: Vec<Vec<Color>>,
    state: ChainState,
    players_alive: Vec<bool>,
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
            state: ChainState::Empty,
            player_count: 2,
            /// Colors - 1 long
            players_alive: vec![true; 5],
            has_boom: false,
            launchpad
        }
    }

    fn render_menu(&mut self) -> Result<(), Box<dyn Error>> {
        self.launchpad.set_session(7, 8, lpx_color!(6))?;
        self.launchpad.set_session(6, 8, lpx_color!(7))?;
        self.launchpad.set_session(5, 8, lpx_color!(10))?;

            match self.state {
                ChainState::Empty => {
                    for i in 0..4 {
                        self.launchpad.set(i, 8, self.colors[0][0])?;
                    }
                    for i in 0..self.player_count {
                        self.launchpad.set(8, i, self.colors[i as usize+1][3])?;
                    }
                    for i in self.player_count..8 {
                        self.launchpad.set(8, i, self.colors[0][0])?;
                    }
                    self.launchpad.set(4, 8, self.colors[self.player_count as usize][3])?;
                    self.launchpad.set(8, 8, self.colors[1][3])?;
                }
                ChainState::Starting(player) |
                ChainState::InProgress(player) => {
                    for i in 0..4 {
                        self.launchpad.set(i, 8, self.colors[0][0])?;
                    }
                    for i in 0..3 {
                        self.launchpad.set(8, i, self.colors[player as usize][i as usize+1])?;
                    }
                    for i in 3..8 {
                        self.launchpad.set(8, i, self.colors[0][0])?;
                    }
                    self.launchpad.set(4, 8, lpx_color!(0))?;
                    self.launchpad.set(8, 8, self.colors[player as usize][3])?;
                    
                }
                ChainState::GameOver(player) => {
                    for i in 0..4 {
                        self.launchpad.set(i, 8, self.colors[player as usize][3])?;
                    }
                    for i in 0..3 {
                        self.launchpad.set(8, i, self.colors[player as usize][i as usize+1])?;
                    }
                    for i in 3..8 {
                        self.launchpad.set(8, i, self.colors[0][0])?;
                    }
                    self.launchpad.set(4, 8, lpx_color!(0))?;
                    self.launchpad.set(8, 8, self.colors[player as usize][3])?;
                }
            }

        Ok(())
    }

    fn reset(&mut self) -> Result<(), Box<dyn Error>> {
        for row in 0..=7 {
            for col in 0..=7 {
                self.board[row][col] = Field::default();
                self.render(row, col)?;
            }
        }

        self.state = ChainState::Empty;
        self.render_menu()?;
        
        Ok(())
    }

    fn render(&self, row: usize, col: usize) -> Result<(), Box<dyn Error>> {
        let item = self.board[row][col];

        let mut color = self.colors[item.player() as usize][min(item.count() as usize, 3)];
        let explosion = match (row % 7, col % 7) {
            (0, 0) => 2,
            (0, _) | (_, 0) => 3,
            _ => 4
        };
        if item.count() >= explosion - 1 {
            color.pulse_mode = PulseMode::Pulse
        }

        self.launchpad.set(col as u8, row as u8, color)?;
        Ok(())
    }

    fn render_new(&self, row: usize, col: usize, field: Field) -> Result<(), Box<dyn Error>> {
        let item = field;
        let mut color = self.colors[item.player() as usize][min(item.count() as usize, 3)];
        let explosion = match (row % 7, col % 7) {
            (0, 0) => 2,
            (0, _) | (_, 0) => 3,
            _ => 4
        };
        if item.count() >= explosion - 1 {
            color.pulse_mode = PulseMode::Pulse
        }
        self.launchpad.set(col as u8, row as u8, color)?;
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

        for x in self.players_alive.iter_mut() {
            *x = false;
        }

        for row in self.board.iter() {
            for field in row.iter() {
                if field.player() > 0 {
                    self.players_alive[field.player() as usize - 1] = true;
                }
            }
        }

        if matches!(self.state, ChainState::InProgress(_)) {
            let mut is_winner = None;

            for (i, x) in self.players_alive.iter().enumerate() {
                if *x {
                    if is_winner.is_some() {
                        is_winner = None;
                        break;
                    } else {
                        is_winner = Some(i+1);
                    }
                }
            }

            if let Some(i) = is_winner {
                self.state = ChainState::GameOver(i as u8);
            }
        }

        self.skip_to_next_player();
        self.render_menu()?;

        Ok(())
    }

    fn step_next_player(&mut self) {
        self.state = match self.state {
            ChainState::Empty => ChainState::Starting(2),
            ChainState::InProgress(x) if x == self.player_count => ChainState::InProgress(1),
            ChainState::Starting(x) if x == self.player_count => ChainState::InProgress(1),
            ChainState::Starting(x) => ChainState::Starting(x+1),
            ChainState::InProgress(x) => ChainState::InProgress(x+1),
            ChainState::GameOver(x) => ChainState::GameOver(x)
        };

        self.skip_to_next_player();
    }

    fn skip_to_next_player(&mut self) {
        let mut next_player = if let ChainState::InProgress(player) = self.state {
            player
        } else {
            return
        };

        while !self.players_alive[next_player as usize - 1] {
            next_player = (next_player % self.player_count) + 1;
        }

        self.state = ChainState::InProgress(next_player);
    }

    fn step(&mut self, row: u8, col: u8) -> bool {
        let item = &mut self.board[row as usize][col as usize];

        let next_player = match self.state {
            ChainState::Empty => 1,
            ChainState::Starting(player) | ChainState::InProgress(player) => player,
            ChainState::GameOver(_) => return false
        };

        let player = item.player();
        let change = match self.state {
            ChainState::GameOver(_) => false,
            _ => player == next_player || player == 0
        };

        if change {
            item.add_count(1);
            item.set_player(next_player);

            self.step_next_player();
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

        let mut tick_time = Duration::from_millis(600);

        loop {
            match midi_in.recv_timeout(Duration::from_millis(50)) {
                Ok(MidiMessage { msg_type: MessageType::CC, key: 98, velocity: vel, .. }) => {
                    if vel > 0 {
                        self.launchpad.clear()?;
                        break;
                    }
                },

                Ok(MidiMessage { msg_type: MessageType::CC, key: 97, velocity: vel, .. }) => {
                    if vel > 0 {
                        self.reset()?;
                    }
                },

                Ok(MidiMessage { msg_type: MessageType::CC, key: 96, velocity: vel, .. }) => {
                    tick_time = if vel > 0 {
                        Duration::from_millis(50)
                    } else {
                        Duration::from_millis(600)
                    }
                },

                Ok(MidiMessage { msg_type: MessageType::CC, key: 95, velocity: vel, .. }) if matches!(self.state, ChainState::Empty) => {
                    if vel > 0 {
                        self.player_count = if self.player_count as usize == self.colors.len() - 1 {
                            2
                        } else {
                            self.player_count + 1
                        };
                        self.render_menu()?;
                    }
                },

                Ok(msg) if matches!(msg.msg_type, MessageType::NoteOn) && !self.has_boom => {
                    if let Some((row, col)) = midi_to_item(&msg) {
                        if msg.velocity > 0 {
                            self.step(row, col);
                            self.launchpad.set(col, row, lpx_color!(36))?;
                            self.render_menu()?;
                        } else {
                            self.render(row as usize, col as usize)?;
                        }
                    }
                }
                _ => ()
            }

            let new_time = Instant::now();

            if new_time - time > tick_time {
                time = new_time;
                self.tick()?;
            }
        }

        self.launchpad.set_programmer_mode(false)?;
        Ok(())
    }
}