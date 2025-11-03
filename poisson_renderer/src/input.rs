use std::collections::HashMap;
use std::sync::{Arc, Weak};
use winit::event::{KeyEvent, WindowEvent};
use winit::keyboard::PhysicalKey;

pub trait ProcessEvent {
    // this is non-mutable
    // everything changed by this has to be in an arc mutex
    fn process_event(self: &Self, event: &WindowEvent);
}

pub struct Input {
    key_state: HashMap<PhysicalKey, bool>,
    str_to_key: HashMap<String, Vec<PhysicalKey>>,
    mouse_position: (f32, f32),
    event_listeners: Vec<Weak<dyn ProcessEvent>>
}

impl Input {
    pub fn new() -> Self {
        Self {
            key_state: HashMap::new(),
            str_to_key: HashMap::new(),
            mouse_position: (0f32, 0f32),
            event_listeners: Vec::new(),
        }
    }
    pub fn process_event(self: &mut Self, event: &WindowEvent) {
        match event {
            WindowEvent::Focused(_) => {}
            WindowEvent::KeyboardInput {
                event: KeyEvent {
                    physical_key: key,
                    state: s, .. 
                }, .. 
            } => {
                if let Some(pressed) = self.key_state.get_mut(&key) {
                    *pressed = s.is_pressed();
                }
            }
            WindowEvent::MouseWheel { .. } => {}
            _ => {}
        }
        
        for l in self.event_listeners.iter() {
            l.upgrade().unwrap().process_event(&event)
        }
    }
    
    pub fn set_mapping(self: &mut Self, name: &str, keys: Vec<PhysicalKey>) {
        //TODO: what happens if some keys are unmapped?
        for key in keys.iter() {
            if !self.key_state.contains_key(&key) {
                self.key_state.insert(*key, false);
            }
        }
        self.str_to_key.insert(name.to_string(), keys);
    }
    
    pub fn is_pressed(self: &Self, name: &str) -> bool {
        for key in self.str_to_key[&name.to_string()].iter() {
            if self.key_state[key] {
                return true;
            }
        };
        
        false
    }
}

