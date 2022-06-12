use std::collections::HashSet;

use crate::log::LOGGER;

/// Handler containing all SDL states needed to process inputs.
/// 
/// Explicit lifetime is to prevent cloning of `keys` HashMap entry key `Keycode`s every time a key is pressed, 
/// and should live as long as the current SDL instance.
pub struct InputDevice {
    game_controller: Option<sdl2::controller::GameController>,
    //joystick: Option<sdl2::joystick::Joystick>,
    //haptic: Option<sdl2::haptic::Haptic>,

    keys_prev: HashSet<sdl2::keyboard::Keycode>,
    keys_old: HashSet<sdl2::keyboard::Keycode>,
    keys_new: HashSet<sdl2::keyboard::Keycode>,

    mouse_buttons_prev: HashSet<sdl2::mouse::MouseButton>,
    mouse_buttons_old: HashSet<sdl2::mouse::MouseButton>,
    mouse_buttons_new: HashSet<sdl2::mouse::MouseButton>,

    mouse_pos: (i32, i32),
    mouse_pos_last: (i32, i32),
    mouse_offset: (i32, i32),
}

impl InputDevice {
    pub fn new(sdl_ctx: &sdl2::Sdl) -> InputDevice {
        InputDevice{
            game_controller: InputDevice::init_controller(sdl_ctx),
            //joystick: init_joystick(),
            //haptic: init_haptic(),

            keys_prev: HashSet::new(),
            keys_old: HashSet::new(),
            keys_new: HashSet::new(),
            
            mouse_buttons_prev: HashSet::new(),
            mouse_buttons_old: HashSet::new(),
            mouse_buttons_new: HashSet::new(),

            mouse_pos: (0, 0),
            mouse_pos_last: (0, 0),
            mouse_offset: (0, 0),
        }
    }

    pub fn process_keymap(&mut self, event_pump: &sdl2::EventPump) {
        let keys = event_pump
            .keyboard_state()
            .pressed_scancodes()
            // Scancodes are physical (independent of keyboard layouts), we need virtualized keys, so convert here
            .filter_map(sdl2::keyboard::Keycode::from_scancode)
            .collect();
        
        self.keys_new = &keys - &self.keys_prev;
        self.keys_old = &self.keys_prev - &keys;

        if !self.keys_new.is_empty() || !self.keys_old.is_empty() {
            LOGGER().a.debug(format!("new_keys: {:?}\told_keys:{:?}", self.keys_new, self.keys_old).as_str());
        }

        self.keys_prev = keys;
    }
    
    pub fn process_mousemap(&mut self, event_pump: &sdl2::EventPump) {
        let mouse_state = event_pump.mouse_state();
        let mouse_buttons = mouse_state.pressed_mouse_buttons().collect();

        self.mouse_buttons_new = &mouse_buttons - &self.mouse_buttons_prev;
        self.mouse_buttons_old = &self.mouse_buttons_prev - &mouse_buttons;

        if !self.mouse_buttons_new.is_empty() || !self.mouse_buttons_old.is_empty() {
            LOGGER().a.debug(
                format!("X = {:?}, Y = {:?}, : {:?} -> {:?}",
                    mouse_state.x(),
                    mouse_state.y(),
                    self.mouse_buttons_new,
                    self.mouse_buttons_old
            ).as_str());
        }

        self.mouse_buttons_prev = mouse_buttons;

        //self.moffset = (self.mpos.0 - self.mpos_last.0, self.mpos_last.1 - self.mpos.1);
        //self.mpos_last = (self.mpos.0, self.mpos.1);
    }

    fn init_controller(sdl_ctx: &sdl2::Sdl) -> Option<sdl2::controller::GameController> {
        let game_controller_subsys = sdl_ctx.game_controller().unwrap();
        let num_controllers_and_joysticks: u32 = match game_controller_subsys.num_joysticks() {
            Err(e) => {
                LOGGER().a.error(format!("can't enumerate joysticks: {}", e).as_str());
                return None;
            },
            Ok(n) => n
        };
        
        LOGGER().a.debug(format!("{} joysticks available", num_controllers_and_joysticks).as_str());

        let controller = (0..num_controllers_and_joysticks)
            .find_map(|id| {
                if !game_controller_subsys.is_game_controller(id) {
                    return None;
                }

                match game_controller_subsys.open(id) {
                    Ok(c) => {
                        LOGGER().a.debug(format!("opened controller '{}'", c.name()).as_str());
                        Some(c)
                    },
                    Err(e) => {
                        LOGGER().a.error(format!("couldn't open controller: {}", e).as_str());
                        None
                    }
                }
            });
        
        match controller {
            Some(c) => {
                LOGGER().a.debug(format!("controller mapping: {}", c.mapping()).as_str());
                Some(c)
            },
            None => {
                LOGGER().a.error("couldn't open any controller!");
                None
            }
        }
    }
}

impl Drop for InputDevice {
    fn drop(&mut self) {
        if self.game_controller.is_some() {
            
        }
    }
}