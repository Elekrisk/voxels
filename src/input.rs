use bevy_ecs::system::Resource;
use cgmath::{Vector2, Zero};
use wgpu::naga::FastHashSet;
use winit::{event::{ElementState, KeyEvent, MouseButton}, keyboard::{KeyCode, PhysicalKey}};

#[derive(Resource)]
pub struct Input {
    pressed_keys: FastHashSet<KeyCode>,
    pressed_mouse_buttons: FastHashSet<MouseButton>,
    just_pressed_keys: FastHashSet<KeyCode>,
    just_pressed_mouse_buttons: FastHashSet<MouseButton>,
    mouse_delta: Vector2<f32>,
}

impl Input {
    pub fn new() -> Self {
        Self {
            pressed_keys: FastHashSet::default(),
            pressed_mouse_buttons: FastHashSet::default(),
            just_pressed_keys: FastHashSet::default(),
            just_pressed_mouse_buttons: FastHashSet::default(),
            mouse_delta: Vector2::zero(),
        }
    }

    pub fn process_key_event(&mut self, event: KeyEvent) {
        if let KeyEvent { physical_key: PhysicalKey::Code(key), state, repeat,  .. } = event {
            if state.is_pressed() {
                if !repeat {
                    self.just_pressed_keys.insert(key);
                }
                self.pressed_keys.insert(key);
            } else {
                self.pressed_keys.remove(&key);
            }
        }
    }

    pub fn process_mouse_move(&mut self, delta: Vector2<f32>) {
        self.mouse_delta += delta;
    }

    pub fn process_mouse_input(&mut self, button: MouseButton, state: ElementState) {
        if state.is_pressed() {
            self.just_pressed_mouse_buttons.insert(button);
            self.pressed_mouse_buttons.insert(button);
        } else {
            self.pressed_mouse_buttons.remove(&button);
        }
    }

    pub fn mouse_delta(&self) -> Vector2<f32> {
        self.mouse_delta
    }

    pub fn reset_mouse_delta(&mut self) {
        self.mouse_delta = Vector2::zero();
    }

    pub fn is_pressed(&self, key: KeyCode) -> bool {
        self.pressed_keys.contains(&key)
    }

    pub fn is_just_pressed(&self, key: KeyCode) -> bool {
        self.just_pressed_keys.contains(&key)
    }

    pub fn is_released(&self, key: KeyCode) -> bool {
        !self.pressed_keys.contains(&key)
    }

    pub fn is_mouse_pressed(&self, button: MouseButton) -> bool {
        self.pressed_mouse_buttons.contains(&button)
    }

    pub fn is_mouse_just_pressed(&self, button: MouseButton) -> bool {
        self.just_pressed_mouse_buttons.contains(&button)
    }

    pub fn end_frame(&mut self) {
        self.reset_mouse_delta();
        self.just_pressed_keys.clear();
        self.just_pressed_mouse_buttons.clear();
    }
}
