// simplest game you can write with graphics, sound and user interaction
extern crate piston_window;
extern crate music;
extern crate find_folder;
extern crate rand;
extern crate time;

use piston_window::*;
use rand::Rng;

const BAT_WIDTH: f64 = 130.0;
const BALL_RADIUS: f64 = 10.0;
const BAT_THICKNESS: f64 = 19.0;
const COURT_HEIGHT: u32 = 700;
const COURT_WIDTH: u32 = 900;
const Y_VEL: f64 = 300.0;
const X_VEL_SHALLOW: f64 = Y_VEL * 1.5;
const X_VEL_NORMAL: f64 = Y_VEL;
const X_VEL_STEEP: f64 = Y_VEL * 0.5;
const MAX_BALLS: usize = 5;
const NEW_BALL_INTERVAL: u32 = 5;

const BAT_Y: f64 = COURT_HEIGHT as f64 - 50.0;
const BALL_RADIUS_SQ: f64 = BALL_RADIUS * BALL_RADIUS;

#[derive(Copy, Clone, Hash, PartialEq, Eq)]
enum Sound {
    HitTable,
    HitBat,
}

#[derive(Copy, Clone, Hash, PartialEq, Eq)]
enum Music {}

enum GameMode {
    StartUp,
    Playing,
    GameOver,
}

struct Score {
    points: u32,
    need_new_ball: bool,
}

impl Score {
    fn new() -> Score {
        Score {points: 0, need_new_ball: false}
    }

    fn reset(&mut self) {
        self.points = 0;
        self.need_new_ball = false;
    }

    fn increment(&mut self) {
        self.points += 1;
        self.need_new_ball = self.points % NEW_BALL_INTERVAL == 0;
    }

    fn new_ball_tripped(&mut self) -> bool {
        let result = self.need_new_ball;
        self.need_new_ball = false;
        result
    }

    fn render(&self, c: Context, g: &mut G2d, glyphs: &mut Glyphs) {
        text::Text::new_color([0.3, 0.3, 0.3, 1.0], 128).draw(
            &self.points.to_string(),
            glyphs,
            &c.draw_state,
            c.transform.trans(((COURT_WIDTH / 2) - 50) as f64, (COURT_HEIGHT / 2) as f64),
            g
            ).unwrap();
    }
}

fn square(n: f64) -> f64 {
    n * n
}

struct Ball {
    x: f64,
    y: f64,
    old_x: f64,
    old_y: f64,
    x_vel: f64,
    y_vel: f64,
    colour: [f32; 4],
    in_play: bool,
}

impl Ball {
    fn new(colour: [f32; 4]) -> Ball {
        Ball{x: 0.0, y: 0.0, old_x: 0.0, old_y: 0.0, x_vel: 0.0, y_vel: 0.0,
            colour, in_play: false}
    }

    fn strike(coord: f64, orth_coord: f64,
              old_coord: f64, old_orth_coord: f64,
              strike_edge: f64, edge_start: f64, edge_end:f64) -> Option<f64> {
        let adj_strike_edge = if coord > old_coord {strike_edge - BALL_RADIUS}
                              else {strike_edge + BALL_RADIUS};
        if (coord > old_coord && coord >= adj_strike_edge) ||
           (coord < old_coord && coord <= adj_strike_edge) {
            let strike_pos = (adj_strike_edge - old_coord) * (orth_coord - old_orth_coord) /
                             (coord - old_coord) + old_orth_coord;
            if strike_pos >= edge_start && strike_pos <= edge_end {
                return Some(strike_pos);
            }
        }
        return None;
    }

    fn normal_rebound(&mut self, from_x: f64, from_y: f64, to_x: f64, to_y: f64) -> bool {
        if from_x == to_x {
            // vertical surface
            match Ball::strike(self.x, self.y, self.old_x, self.old_y, from_x, from_y, to_y) {
                Some(_) => {
                    self.x_vel = - self.x_vel;
                    self.x = 2.0 * from_x - self.x + (self.x_vel.signum() * 2.0 * BALL_RADIUS);
                    true
                },
                _ => {
                    false
                },
            }
        }
        else {
            // horizontal surface
            match Ball::strike(self.y, self.x, self.old_y, self.old_x, from_y, from_x, to_x) {
                Some(_) => {
                    self.y_vel = - self.y_vel;
                    self.y = 2.0 * from_y - self.y + (self.y_vel.signum() * 2.0 * BALL_RADIUS);
                    true
                },
                _ => {
                    false
                },
            }
        }
    }

    fn bat_face_rebound(&mut self, bat_x: f64, speedup: f64) -> bool {
        match Ball::strike(self.y, self.x, self.old_y, self.old_x, BAT_Y, bat_x, bat_x + BAT_WIDTH) {
            Some(strike_x) => {
                // these are not the traditional pong bat rebound rules...
                let face_pos = if self.x_vel > 0.0 {strike_x - bat_x}
                               else {bat_x + BAT_WIDTH - strike_x};
                let y_overshoot = self.y - BAT_Y + BALL_RADIUS;
                let p_overshoot = y_overshoot / self.y_vel;
                if face_pos < 0.3 * BAT_WIDTH {
                    self.x_vel = self.x_vel.signum() * X_VEL_STEEP * speedup;
                }
                else if face_pos > 0.7 * BAT_WIDTH {
                    self.x_vel = self.x_vel.signum() * X_VEL_SHALLOW * speedup;
                }
                else {
                    self.x_vel = self.x_vel.signum() * X_VEL_NORMAL * speedup;
                }
                self.y = BAT_Y - BALL_RADIUS - y_overshoot;
                self.x = strike_x + (self.x_vel * p_overshoot);
                self.y_vel = - Y_VEL * speedup;
                true
            },
            _ => {
                false
            }
        }
    }

    fn bat_corner_rebound(&mut self, bat_x: f64, speedup: f64) -> bool {
        // we assume face hit has already been ruled out
        // this is a "naive" implementation of corner striking
        if self.y < BAT_Y {
            // left bat corner
            if square(self.x - bat_x) + square(self.y - BAT_Y) < BALL_RADIUS_SQ {
                self.y_vel = - Y_VEL * speedup;
                self.x_vel = - X_VEL_SHALLOW * speedup;
                return true;
            }
            // right bat corner
            if square(self.x - bat_x - BAT_WIDTH) + square(self.y - BAT_Y) < BALL_RADIUS_SQ {
                self.y_vel = - Y_VEL * speedup;
                self.x_vel =  X_VEL_SHALLOW * speedup;
                return true;
            }
        }
        return false;
    }

    fn bat_sound() {
        music::play_sound(&Sound::HitBat, music::Repeat::Times(0), music::MAX_VOLUME);
    }

    fn table_sound() {
        music::play_sound(&Sound::HitTable, music::Repeat::Times(0), music::MAX_VOLUME);
    }

    fn update(&mut self, dt: f64, bat_x: f64, score: &mut Score, speedup: f64) {
        if self.in_play {
            self.old_x = self.x;
            self.old_y = self.y;
            self.x += &self.x_vel * dt;
            self.y += &self.y_vel * dt;

            if self.x_vel > 0.0 &&
                self.normal_rebound(COURT_WIDTH as f64, -100.0,
                    COURT_WIDTH as f64, COURT_HEIGHT as f64 + 100.0) {
                Ball::table_sound();
            }
            else if self.x_vel < 0.0 &&
              self.normal_rebound(0.0, -100.0, 0.0, COURT_HEIGHT as f64 + 100.0) {
                Ball::table_sound();
            }
            if self.y_vel < 0.0 &&
              self.normal_rebound(-100.0, 0.0, COURT_WIDTH as f64 + 100.0, 0.0) {
                Ball::table_sound();
            }
            else if self.y_vel > 0.0 && self.old_y < BAT_Y - BALL_RADIUS &&
                self.bat_face_rebound(bat_x, speedup) {
                Ball::bat_sound();
                score.increment();
            }
            if self.y >= BAT_Y && self.x_vel > 0.0 &&
                self.normal_rebound(bat_x, BAT_Y, bat_x, BAT_Y + BAT_THICKNESS) {
                Ball::bat_sound();
            }
            else if self.y >= BAT_Y && self.x_vel < 0.0 &&
                self.normal_rebound(bat_x + BAT_WIDTH, BAT_Y, bat_x + BAT_WIDTH,
                                    BAT_Y + BAT_THICKNESS) {
                Ball::bat_sound();
            }
            else if self.bat_corner_rebound(bat_x, speedup) {
                Ball::bat_sound();
                score.increment();
            }
            if self.y > COURT_HEIGHT as f64 + BALL_RADIUS {
                self.in_play = false;
            }
        }
    }

    fn serve(&mut self, speedup: f64) {
        self.x_vel = (if rand::thread_rng().gen() {X_VEL_NORMAL} else {- X_VEL_NORMAL}) * speedup;
        self.y_vel = Y_VEL * speedup;
        self.x = rand::thread_rng().gen_range(BALL_RADIUS as u32 + 1, COURT_WIDTH - BALL_RADIUS as u32) as f64;
        self.y = rand::thread_rng().gen_range(BALL_RADIUS as u32 + 1, COURT_HEIGHT / 3) as f64;
        self.old_x = self.x - self.x_vel;
        self.old_y = self.y - self.y_vel;
        self.in_play = true;
    }

    fn render(&self, c: Context, g: &mut G2d) {
        if self.in_play {
            let ball_shape = ellipse::circle(0.0, 0.0, BALL_RADIUS);
            ellipse(self.colour, ball_shape, c.transform.trans(self.x, self.y), g);
        }
    }
}

struct Balls {
    ball: [Ball; MAX_BALLS]
}

impl Balls {
    fn new() -> Balls {
         Balls {ball: [Ball::new([1.0, 1.0, 1.0, 1.0]),
                       Ball::new([0.9, 0.65, 0.89, 1.0]),
                       Ball::new([0.9, 0.71, 0.5, 1.0]),
                       Ball::new([0.75, 0.91, 0.63, 1.0]),
                       Ball::new([0.73, 0.89, 0.9, 1.0])
         ]}
    }

    fn update(&mut self, dt: f64, bat_x: f64, score: &mut Score, speedup: f64) {
        for i in 0..MAX_BALLS {
            self.ball[i].update(dt, bat_x, score, speedup);
        }
        if score.new_ball_tripped() {
            self.serve_new_ball(speedup);
        }
    }

    fn render(&self, c: Context, g: &mut G2d) {
        for i in 0..MAX_BALLS {
            self.ball[i].render(c, g);
        }
    }

    fn serve_new_ball(&mut self, speedup: f64) {
        for i in 0..MAX_BALLS {
            if ! self.ball[i].in_play {
                self.ball[i].serve(speedup);
                break;
            }
        }
    }

    fn in_play(&self) -> bool {
        for i in 0..MAX_BALLS {
            if self.ball[i].in_play {
                return true;
            }
        }
        return false;
    }
}

fn render_instruction(c: Context, g: &mut G2d, glyphs: &mut Glyphs, x: f64, y: f64, txt: &str) {
    text::Text::new_color([1.0, 1.0, 1.0, 1.0], 32).draw(
        txt,
        glyphs,
        &c.draw_state,
        c.transform.trans(x, y),
        g
        ).unwrap();
}

fn main() {
    let opengl = OpenGL::V2_1;
    let mut window: PistonWindow = WindowSettings::new(
        "Pongish",
        (COURT_WIDTH, COURT_HEIGHT)
    )
    .exit_on_esc(true)
    .opengl(opengl)
    .build()
    .unwrap();

    window.set_capture_cursor(true);

    let assets = find_folder::Search::ParentsThenKids(3, 3)
        .for_folder("assets").unwrap();

    // assets - images
    let bat_image = assets.join("bat.png");
    let bat_image = Texture::from_path(
            &mut window.factory,
            &bat_image,
            Flip::None,
            &TextureSettings::new()
        ).unwrap();

    // assets - sounds
    let hit_table = assets.join("hit-table.wav");
    let hit_bat = assets.join("hit-bat.wav");

    // assets - font
    let ref font = assets.join("FiraSans-Regular.ttf");
    let factory = window.factory.clone();
    let mut glyphs = Glyphs::new(font, factory, TextureSettings::new()).unwrap();

    let mut bat_x = (COURT_WIDTH as f64 + BAT_WIDTH) / 2.0;
    let mut score = Score::new();
    let mut game_mode = GameMode::StartUp;

    let mut balls = Balls::new();

    let mut accumt: f64 = 0.0;
    let mut start_time = time::Timespec {sec: 0, nsec: 0};
    let mut end_time  = time::Timespec {sec: 0, nsec: 0};

    // need to rein in ups as performs poorly on my hardware
    // delta time supplied by update_args does not adjust to actual time elapsed but what
    // it was supposed to be according to ups - if we overrun we get slow down
    window.set_ups(60); 

    #[cfg(debug_assertions)]
    window.set_ups(20); 

    music::start::<Music, Sound, _>(16, || {
        music::bind_sound_file(Sound::HitTable, hit_table);
        music::bind_sound_file(Sound::HitBat, hit_bat);

        while let Some(e) = window.next() {
            match game_mode {
                GameMode::StartUp => {
                    window.draw_2d(&e, |c, g| {
                        clear([0.0, 0.0, 0.0, 1.0], g);

                        if let Some(_) = e.render_args() {
                            // pre-render all digits as they take longer the first time
                            for digit in 0..10 {
                                text::Text::new_color([0.3, 0.3, 0.3, 1.0], 128).draw(
                                    &digit.to_string(),
                                    &mut glyphs,
                                    &c.draw_state,
                                    c.transform.trans(-200.0, -200.0),
                                    g
                                    ).unwrap();
                            }

                            render_instruction(c, g, &mut glyphs, 200.0, 400.0,
                                               "Pongish - somewhat Pong-like");
                            render_instruction(c, g, &mut glyphs, 200.0, 475.0,
                                               "Press space to play");
                            render_instruction(c, g, &mut glyphs, 200.0, 550.0,
                                               "Use the mouse to control the bat");
                            render_instruction(c, g, &mut glyphs, 200.0, 625.0,
                                               "Press escape to exit at any time");
                        }
                    });
                    if let Some(Button::Keyboard(Key::Space)) = e.release_args() {
                        game_mode = GameMode::Playing;
                        score.reset();
                        balls.ball[0].serve(1.0);
                        accumt = 0.0;
                        start_time = time::get_time();
                    }
                },
                GameMode::Playing => {
                    if let Some(_) = e.render_args() {
                        window.draw_2d(&e, |c, g| {
                            clear([0.0, 0.0, 0.0, 1.0], g);
                            score.render(c, g, &mut glyphs);
                            balls.render(c, g);
                            image(&bat_image, c.transform.trans(bat_x, BAT_Y), g);
                        });
                    }

                    e.mouse_relative(|x, _| {
                        bat_x += x;
                        if bat_x < - BAT_WIDTH {
                            bat_x = - BAT_WIDTH;
                        }
                        if bat_x > COURT_WIDTH as f64 {
                            bat_x = COURT_WIDTH as f64;
                        }
                    });

                    if let Some(u) = e.update_args() {
                        balls.update(u.dt, bat_x, &mut score, 1.0 + (accumt / 200.0));
                        accumt += u.dt;

                        if ! balls.in_play() {
                            game_mode = GameMode::GameOver;
                            end_time = time::get_time();
                            let elapsed_time: f64 = (end_time.sec - start_time.sec) as f64 +
                                (end_time.nsec as f64 / 1000000000.0) -
                                (start_time.nsec as f64 / 1000000000.0);
                            println!("Lost time: {} of {}", elapsed_time - accumt, elapsed_time);
                        }
                    }

                },
                GameMode::GameOver => {
                    if let Some(_) = e.render_args() {
                        window.draw_2d(&e, |c, g| {
                            clear([0.0, 0.0, 0.0, 1.0], g);
                            score.render(c, g, &mut glyphs);
                            render_instruction(c, g, &mut glyphs, 200.0, 400.0,
                                               "Game Over");
                            render_instruction(c, g, &mut glyphs, 200.0, 475.0,
                                               "Press space to play again");
                            render_instruction(c, g, &mut glyphs, 200.0, 550.0,
                                               "Press escape to exit at any time");
                        });
                    }
                    if let Some(Button::Keyboard(Key::Space)) = e.release_args() {
                        game_mode = GameMode::Playing;
                        score.points = 0;
                        balls.ball[0].serve(1.0);
                        accumt = 0.0;
                        start_time = time::get_time();
                    }
                },
            }
        }
    });
}
