use bracket_lib::prelude::*;
use rodio::{Decoder, OutputStream, Sink, Source};
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;

const SCREEN_WIDTH : i32 = 80;
const SCREEN_HEIGHT : i32 = 50;
const START_X: i32 = 5;
const FRAME_DURATION : f32 = 75.0;
const CHICKEN_FRAMES : [u16; 6] = [ 64, 1, 2, 3, 2, 1 ];

fn resource_path(file_name: &str) -> PathBuf {
    let mut candidates = Vec::new();

    if let Ok(current_dir) = std::env::current_dir() {
        candidates.push(current_dir.join("resources").join(file_name));
        candidates.push(current_dir.join("..").join("resources").join(file_name));
    }

    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            candidates.push(exe_dir.join("resources").join(file_name));
            candidates.push(exe_dir.join("..").join("resources").join(file_name));
            candidates.push(exe_dir.join("..").join("..").join("resources").join(file_name));
        }
    }

    candidates
        .into_iter()
        .find(|path| path.exists())
        .unwrap_or_else(|| PathBuf::from("resources").join(file_name))
}

struct Music {
    _stream: OutputStream,
    sink: Sink,
}

impl Music {
    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let (_stream, stream_handle) = OutputStream::try_default()?;
        let sink = Sink::try_new(&stream_handle)?;
        let song = BufReader::new(File::open(resource_path("chicken-wing-song_gKkgSVP.mp3"))?);

        sink.append(Decoder::new(song)?.repeat_infinite());
        sink.pause();

        Ok(Self { _stream, sink })
    }

    fn play(&self) {
        self.sink.play();
    }

    fn pause(&self) {
        self.sink.pause();
    }
}

enum GameMode {
    Menu,
    Playing,
    End,
}

struct Player {
    x: i32,
    y: f32,
    velocity: f32,
    frame: usize,
}

impl Player {
    fn new(x: i32, y: i32) -> Self {
        Player {
            x,
            y: y as f32,
            velocity: 0.0,
            frame: 0,
        }
    }

    fn render(&mut self, ctx: &mut BTerm) {
        ctx.set_active_console(1);
        ctx.cls();
        ctx.set_fancy(
            PointF::new(0.0 + START_X as f32, self.y),
            1,
            Degrees::new(0.0),
            PointF::new(2.0, 2.0),
            WHITE,
            NAVY,
            CHICKEN_FRAMES[self.frame]
        );
        ctx.set_active_console(0);
    }

    fn gravity_and_move(&mut self) {
        if self.velocity < 2.0 {
            self.velocity += 0.2;
        }
        self.y += self.velocity;
        if self.y < 0.0 {
            self.y = 0.0;
        }
        self.x += 1;
        self.frame += 1;
        self.frame = self.frame % 6;
    }

    fn flap(&mut self) {
        self.velocity = -2.0;
    }
}

struct Obstacle {
    x: i32,
    gap_y: i32,
    size: i32
}

impl Obstacle {
    fn new (x: i32, score: i32) -> Self {
        let mut random = RandomNumberGenerator::new();
        Obstacle {
            x,
            gap_y: random.range(10, 40),
            size: i32::max(2, 20 - score)
        }
    }

    fn render(&mut self, ctx: &mut BTerm, player_x: i32) {
        // The ground
        for x in 0..SCREEN_WIDTH {
            ctx.set(x, SCREEN_HEIGHT-1, WHITE, WHITE, to_cp437('#'));
        }

        let screen_x = self.x - player_x;
        let half_size = self.size / 2;
        // Top wall
        for y in 0..self.gap_y - half_size {
            ctx.set(
                screen_x,
                y,
                WHITE,
                NAVY,
                179,
            );
        }

        // Bottom wall - now leaving room for the ground
        for y in self.gap_y + half_size..SCREEN_HEIGHT - 1 {
            ctx.set(
                screen_x,
                y,
                WHITE,
                NAVY,
                179,
            );
        }
    }

    fn hit_obstacle(&self, player: &Player) -> bool {
        let half_size = self.size / 2;
        player.x == self.x - START_X && 
        ((player.y as i32) < self.gap_y - half_size || player.y as i32 > self.gap_y + half_size)
    }
}

struct State {
    player: Player,
    frame_time: f32,
    obstacle: Obstacle,
    mode: GameMode,
    score: i32,
    music: Option<Music>,
}

impl State {
    fn new() -> Self {
        State {
            player: Player::new(5, 25),
            frame_time: 0.0,
            obstacle: Obstacle::new(SCREEN_WIDTH, 0),
            mode: GameMode::Menu,
            score: 0,
            music: Music::new().ok(),
        }
    }

    fn play(&mut self, ctx: &mut BTerm) {
        ctx.cls_bg(NAVY);
        self.frame_time += ctx.frame_time_ms;

        if self.frame_time > FRAME_DURATION {
            self.frame_time = 0.0;
            self.player.gravity_and_move();
        }

        if let Some(VirtualKeyCode::Space) = ctx.key {
            self.player.flap();
        }

        self.obstacle.render(ctx, self.player.x);
        if self.player.x > self.obstacle.x {
            self.score += 1;
            self.obstacle = Obstacle::new(self.player.x + SCREEN_WIDTH, self.score);
        }
        if self.player.y as i32 > SCREEN_HEIGHT || self.obstacle.hit_obstacle(&self.player) {
            self.mode = GameMode::End;
        }

        self.player.render(ctx);
        ctx.print(0, 0, "Press SPACE to flap.");
        ctx.print(0, 1, &format!("Score: {}", self.score));
        if self.player.y as i32  > SCREEN_HEIGHT {
            self.mode = GameMode::End;
        }
    }

    fn restart(&mut self) {
        self.player = Player::new(5, 25);
        self.frame_time = 0.0;
        self.obstacle = Obstacle::new(SCREEN_WIDTH, 0);
        self.mode = GameMode::Playing;
        self.score = 0;
    }

    fn main_menu(&mut self, ctx: &mut BTerm) {
        ctx.cls();
        ctx.print_centered(5, "Welcome to Flappy Chicken");
        ctx.print_centered(8, "(P) Play Game");
        ctx.print_centered(9, "(Q) Quit Game");

        if let Some(key) = ctx.key {
            match key {
                VirtualKeyCode::P => self.restart(),
                VirtualKeyCode::Q => ctx.quitting = true,
                _ => {}
            }
        }
    }

    fn dead(&mut self, ctx: &mut BTerm) {
        ctx.cls();
        ctx.print_centered(5, "You are dead!");
        ctx.print_centered(6, &format!("You earned {} points", self.score));
        ctx.print_centered(8, "(P) Play Again");
        ctx.print_centered(9, "(Q) Quit Game");

        if let Some(key) = ctx.key {
            match key {
                VirtualKeyCode::P => self.restart(),
                VirtualKeyCode::Q => ctx.quitting = true,
                _ => {}
            }
        }


    }
}

impl GameState for State {
    fn tick(&mut self, ctx: &mut BTerm) {
        if let Some(music) = &self.music {
            match self.mode {
                GameMode::Playing => music.play(),
                GameMode::Menu | GameMode::End => music.pause(),
            }
        }

        match self.mode {
            GameMode::Menu => self.main_menu(ctx),
            GameMode::End => self.dead(ctx),
            GameMode::Playing => self.play(ctx),
        }
    }
}

fn main() -> BError {
    let font_path = resource_path("flappy32.png").to_string_lossy().to_string();
    let context = BTermBuilder::new()
        .with_font(&font_path, 32, 32)
        .with_simple_console(SCREEN_WIDTH, SCREEN_HEIGHT, &font_path)
        .with_fancy_console(SCREEN_WIDTH, SCREEN_HEIGHT, &font_path)
        .with_title("Flappy Chicken")
        .with_tile_dimensions(16, 16)
        .build()?;
    main_loop(context, State::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resources_are_accessible_and_song_decodes() {
        let font_path = resource_path("flappy32.png");
        let song_path = resource_path("chicken-wing-song_gKkgSVP.mp3");

        assert!(font_path.exists(), "missing font resource: {:?}", font_path);
        assert!(song_path.exists(), "missing song resource: {:?}", song_path);

        let song = BufReader::new(File::open(&song_path).expect("song file should open"));
        Decoder::new(song).expect("song file should decode as audio");
    }
}

