//! Game loop.

// External crates.
use std::mem::replace;
use sync::{Mutex, Arc};

// Local crate.
use ConcurrentWindow;
use GameLoopWindow;
use RenderWindow;

use GameEvent;
use GameIteratorSettings;
use ConcurrentIterator;
use KeyPress;
use KeyPressArgs;
use KeyRelease;
use KeyReleaseArgs;
use MouseMove;
use MouseMoveArgs;
use MouseRelativeMove;
use MouseRelativeMoveArgs;
use MousePress;
use MousePressArgs;
use MouseRelease;
use MouseReleaseArgs;
use Render;
use RenderArgs;
use Update;
use UpdateArgs;

/// Implemented by game applications.
pub trait ConcurrentGame<R>: Copy + Send {
    /// Render graphics.
    fn render(&self, _resources: &mut R, _args: &RenderArgs) {}

    /// Update the physical state of the game.
    fn update(&mut self, _args: &UpdateArgs) {}

    /// Perform tasks for loading before showing anything.
    fn load(&mut self) {}

    /// User pressed a key.
    ///
    /// This can be overridden to handle key pressed events.
    fn key_press(&mut self, _args: &KeyPressArgs) {}

    /// User released a key.
    ///
    /// This can be overridden to handle key released events.
    fn key_release(&mut self, _args: &KeyReleaseArgs) {}

    /// Pressed a mouse button.
    fn mouse_press(&mut self, _args: &MousePressArgs) {}

    /// Released a mouse button.
    fn mouse_release(&mut self, _args: &MouseReleaseArgs) {}

    /// Moved mouse cursor.
    fn mouse_move(&mut self, _args: &MouseMoveArgs) {}

    /// Moved mouse relative, not bounded by cursor.
    fn mouse_relative_move(&mut self, _args: &MouseRelativeMoveArgs) {}

    /// Executes a game loop.
    fn run<RW: RenderWindow, GLW: GameLoopWindow, W: ConcurrentWindow<RW, GLW>> (
        mut self,
        combined_window: W,
        game_iter_settings: GameIteratorSettings,
        mut render_resources: R
    ) {

        // Setup.

        self.load();
        let (render_window, game_loop_window) = combined_window.get_windows();
        let mutex_self1 = Arc::new( Mutex::new( self ) );
        let mutex_self2 = mutex_self1.clone();
        let (tx, rx) = channel();

        // Game_loop thread.

        spawn(proc() {
            let mut buf2 = self;
            let mut window = game_loop_window;
            
            let mut game_iter = ConcurrentIterator::new(
                &mut window,
                &game_iter_settings
             );
            
            loop {
                match game_iter.next() {
                    None => break,
                    Some(mut e) => match e {
                        Render(args) => {    
                            let mut mutex_guard = mutex_self2.lock();
                            {
                                let render_buf = mutex_guard.deref_mut();
                                replace( render_buf, buf2 );
                            }
                            mutex_guard.cond.signal();
                            tx.send(args);
                        },
                        Update(ref mut args) => buf2.update(args),
                        KeyPress(ref args) => buf2.key_press(args),
                        KeyRelease(ref args) => buf2.key_release(args),
                        MousePress(ref args) => buf2.mouse_press(args),
                        MouseRelease(ref args) => buf2.mouse_release(args),
                        MouseMove(ref args) => buf2.mouse_move(args),
                        MouseRelativeMove(ref args) => buf2.mouse_relative_move(args),
                    }
                }
            }
        });

        // Rendering thread.

        loop {
            let mut args: RenderArgs = rx.recv();
            let mut mutex_guard = mutex_self1.lock();
            {
                let render_buf = mutex_guard.deref_mut();
                render_buf.render( &mut render_resources, &mut args);
                render_window.swap_buffers();
            }
            mutex_guard.cond.signal();
        }
    }
}
