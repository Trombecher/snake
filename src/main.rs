use crossterm::cursor::{Hide, MoveTo, Show};
use crossterm::event::{poll, read, Event, KeyCode, KeyEventKind};
use crossterm::style::{Print, PrintStyledContent, Stylize};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, size, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::{execute, queue};
use rand::{thread_rng, Rng};
use std::collections::VecDeque;
use std::io;
use std::io::stdout;
use std::mem::transmute;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU8, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{sleep, spawn};
use std::time::Duration;

#[derive(Copy, Clone)]
#[repr(u8)]
enum Direction {
    Right,
    Down,
    Left,
    Up
}

fn main() -> io::Result<()> {
    enable_raw_mode()?;

    execute!(stdout(), EnterAlternateScreen, MoveTo(0, 0), Hide)?;

    let direction = Arc::new(AtomicU8::new(Direction::Right as u8));
    let snake = Arc::new(Mutex::new(VecDeque::<(u16, u16)>::new()));
    snake.lock().unwrap().push_front((0, 0));
    snake.lock().unwrap().push_front((1, 0));

    let (initial_width, initial_height) = size()?;

    let width_and_height = Arc::new(AtomicU32::from((initial_width as u32) | ((initial_height as u32) << 16)));
    let do_loop = Arc::new(AtomicBool::from(true));

    let render_thread = {
        let snake = snake.clone();
        let direction = direction.clone();
        let do_loop = do_loop.clone();
        let width_and_height = width_and_height.clone();

        let mut apple_pos = (10, 10);

        let rt = spawn(move || {
            let mut stdout = stdout().lock();
            let mut rng = thread_rng();
            let mut score = 0_u32;

            while do_loop.load(Ordering::SeqCst) {
                let (real_width, height) = unsafe { transmute::<_, (u16, u16)>(width_and_height.load(Ordering::SeqCst)) };
                let real_width = real_width / 2;

                let direction = unsafe { transmute::<_, Direction>(direction.load(Ordering::SeqCst)) };
                let front = *snake.lock().unwrap().front().unwrap();

                let new_head = match direction {
                    Direction::Right => ((front.0 + 1) % real_width, front.1),
                    Direction::Down => (front.0, (front.1 + 1) % height),
                    Direction::Left => ((front.0 + real_width - 1) % real_width, front.1),
                    Direction::Up => (front.0, (front.1 + height - 1) % height),
                };

                for tail_piece in snake.lock().unwrap().iter().copied() {
                    if tail_piece == new_head {
                        do_loop.store(false, Ordering::SeqCst);
                        continue;
                    }
                }

                snake.lock().unwrap().push_front(new_head);

                if apple_pos != new_head {
                    snake.lock().unwrap().pop_back();
                } else {
                    apple_pos.0 = rng.gen_range(0..real_width / 2);
                    apple_pos.1 = rng.gen_range(0..height);
                    score += 1;
                }

                queue!(
                    stdout,
                    Clear(ClearType::Purge),
                    Clear(ClearType::All),

                    // Print score
                    MoveTo(0, 0),
                    Print("Score: "),
                    Print(score),

                    // Print apple
                    MoveTo(apple_pos.0 * 2, apple_pos.1),
                    PrintStyledContent("()".red())
                ).unwrap();

                for (x, y) in snake.lock().unwrap().iter().copied() {
                    queue!(stdout, MoveTo(x * 2, y), PrintStyledContent("  ".on_white())).unwrap();
                }

                execute!(stdout).unwrap();

                sleep(Duration::from_millis(70))
            }
        });

        rt
    };

    while do_loop.load(Ordering::Relaxed) {
        if !poll(Duration::from_millis(1))? {
            continue;
        }

        match read()? {
            Event::FocusGained => {}
            Event::FocusLost => {}
            Event::Key(key) => {
                if key.kind == KeyEventKind::Release {
                    continue
                }

                match key.code {
                    KeyCode::Left => {
                        direction.store(Direction::Left as u8, Ordering::SeqCst)
                    },
                    KeyCode::Right => direction.store(Direction::Right as u8, Ordering::SeqCst),
                    KeyCode::Up => direction.store(Direction::Up as u8, Ordering::SeqCst),
                    KeyCode::Down => direction.store(Direction::Down as u8, Ordering::SeqCst),
                    KeyCode::Char('q') => {
                        do_loop.store(false, Ordering::SeqCst);
                        break
                    }
                    _ => continue,
                }
            }
            Event::Resize(w, h) => {
                width_and_height.store((w as u32) | ((h as u32) << 16), Ordering::SeqCst);
            }
            _ => {}
        }
    }

    render_thread.join().unwrap();

    execute!(stdout(), LeaveAlternateScreen, Show)?;

    disable_raw_mode()
}