use abstutil;
use ezgui::{Canvas, GfxCtx, InputResult, Menu, TextBox, UserInput};
use geom::Polygon;
use map_model::Map;
use sim::{Neighborhood, Tick};
use std::collections::VecDeque;

pub struct Wizard {
    alive: bool,
    tb: Option<TextBox>,
    string_menu: Option<Menu<()>>,
    neighborhood_menu: Option<Menu<Neighborhood>>,

    state_usize: Vec<usize>,
    state_tick: Vec<Tick>,
    state_percent: Vec<f64>,
    state_choices: Vec<String>,
    state_string: Vec<String>,
}

impl Wizard {
    pub fn new() -> Wizard {
        Wizard {
            alive: true,
            tb: None,
            string_menu: None,
            neighborhood_menu: None,
            state_usize: Vec::new(),
            state_tick: Vec::new(),
            state_percent: Vec::new(),
            state_choices: Vec::new(),
            state_string: Vec::new(),
        }
    }

    pub fn draw(&self, g: &mut GfxCtx, canvas: &Canvas) {
        if let Some(ref menu) = self.string_menu {
            menu.draw(g, canvas);
        }
        if let Some(ref menu) = self.neighborhood_menu {
            menu.draw(g, canvas);
            g.draw_polygon(
                [0.0, 0.0, 1.0, 0.6],
                &Polygon::new(&menu.current_choice().points),
            );
        }
        if let Some(ref tb) = self.tb {
            tb.draw(g, canvas);
        }
    }

    pub fn wrap<'a>(&'a mut self, input: &'a mut UserInput, map: &'a Map) -> WrappedWizard<'a> {
        assert!(self.alive);

        let ready_usize = VecDeque::from(self.state_usize.clone());
        let ready_tick = VecDeque::from(self.state_tick.clone());
        let ready_percent = VecDeque::from(self.state_percent.clone());
        let ready_choices = VecDeque::from(self.state_choices.clone());
        let ready_string = VecDeque::from(self.state_string.clone());
        WrappedWizard {
            wizard: self,
            input,
            map,
            ready_usize,
            ready_tick,
            ready_percent,
            ready_choices,
            ready_string,
        }
    }

    pub fn aborted(&self) -> bool {
        !self.alive
    }

    fn input_with_text_box<R>(
        &mut self,
        query: &str,
        input: &mut UserInput,
        parser: Box<Fn(String) -> Option<R>>,
    ) -> Option<R> {
        assert!(self.alive);

        // Otherwise, we try to use one event for two inputs potentially
        if input.has_been_consumed() {
            return None;
        }

        if self.tb.is_none() {
            self.tb = Some(TextBox::new(query));
        }

        match self.tb.as_mut().unwrap().event(input) {
            InputResult::StillActive => None,
            InputResult::Canceled => {
                self.alive = false;
                None
            }
            InputResult::Done(line, _) => {
                self.tb = None;
                if let Some(result) = parser(line.clone()) {
                    Some(result)
                } else {
                    warn!("Invalid input {}", line);
                    None
                }
            }
        }
    }
}

// Lives only for one frame -- bundles up temporary things like UserInput and statefully serve
// prior results.
pub struct WrappedWizard<'a> {
    wizard: &'a mut Wizard,
    input: &'a mut UserInput,
    // TODO a workflow needs the map name. fine?
    pub map: &'a Map,

    ready_usize: VecDeque<usize>,
    ready_tick: VecDeque<Tick>,
    ready_percent: VecDeque<f64>,
    ready_choices: VecDeque<String>,
    ready_string: VecDeque<String>,
}

impl<'a> WrappedWizard<'a> {
    pub fn input_string(&mut self, query: &str) -> Option<String> {
        if !self.ready_string.is_empty() {
            return self.ready_string.pop_front();
        }
        if let Some(s) = self.wizard.input_with_text_box(
            query,
            self.input,
            Box::new(|line| Some(line)),
        ) {
            self.wizard.state_string.push(s.clone());
            Some(s)
        } else {
            None
        }
    }

    pub fn input_usize(&mut self, query: &str) -> Option<usize> {
        if !self.ready_usize.is_empty() {
            return self.ready_usize.pop_front();
        }
        if let Some(num) = self.wizard.input_with_text_box(
            query,
            self.input,
            Box::new(|line| line.parse::<usize>().ok()),
        ) {
            self.wizard.state_usize.push(num);
            Some(num)
        } else {
            None
        }
    }

    pub fn input_tick(&mut self, query: &str) -> Option<Tick> {
        if !self.ready_tick.is_empty() {
            return self.ready_tick.pop_front();
        }
        if let Some(tick) =
            self.wizard
                .input_with_text_box(query, self.input, Box::new(|line| Tick::parse(&line)))
        {
            self.wizard.state_tick.push(tick);
            Some(tick)
        } else {
            None
        }
    }

    pub fn input_percent(&mut self, query: &str) -> Option<f64> {
        if !self.ready_percent.is_empty() {
            return self.ready_percent.pop_front();
        }
        if let Some(percent) = self.wizard.input_with_text_box(
            query,
            self.input,
            Box::new(|line| {
                line.parse::<f64>().ok().and_then(|num| {
                    if num >= 0.0 && num <= 1.0 {
                        Some(num)
                    } else {
                        None
                    }
                })
            }),
        ) {
            self.wizard.state_percent.push(percent);
            Some(percent)
        } else {
            None
        }
    }

    pub fn choose(&mut self, query: &str, choices: Vec<&str>) -> Option<String> {
        if !self.ready_choices.is_empty() {
            return self.ready_choices.pop_front();
        }

        if self.wizard.string_menu.is_none() {
            self.wizard.string_menu = Some(Menu::new(
                query,
                choices.into_iter().map(|s| (s.to_string(), ())).collect(),
            ));
        }

        if let Some((choice, _)) = input_with_menu(
            &mut self.wizard.string_menu,
            &mut self.wizard.alive,
            self.input,
        ) {
            self.wizard.state_choices.push(choice.clone());
            Some(choice)
        } else {
            None
        }
    }

    pub fn choose_neighborhood(&mut self, query: &str) -> Option<String> {
        if !self.ready_choices.is_empty() {
            return self.ready_choices.pop_front();
        }

        if self.wizard.neighborhood_menu.is_none() {
            self.wizard.neighborhood_menu = Some(Menu::new(
                query,
                abstutil::load_all_objects("neighborhoods", self.map.get_name()),
            ));
        }

        if let Some((name, _)) = input_with_menu(
            &mut self.wizard.neighborhood_menu,
            &mut self.wizard.alive,
            self.input,
        ) {
            self.wizard.state_choices.push(name.clone());
            Some(name)
        } else {
            None
        }
    }
}

// The caller initializes the menu, if needed. Pass in Option that must be Some().
// Bit weird to be a free function, but need to borrow a different menu and also the alive bit.
fn input_with_menu<T: Clone>(
    menu: &mut Option<Menu<T>>,
    alive: &mut bool,
    input: &mut UserInput,
) -> Option<(String, T)> {
    assert!(*alive);

    // Otherwise, we try to use one event for two inputs potentially
    if input.has_been_consumed() {
        return None;
    }

    match menu.as_mut().unwrap().event(input) {
        InputResult::Canceled => {
            *menu = None;
            *alive = false;
            None
        }
        InputResult::StillActive => None,
        InputResult::Done(name, poly) => {
            *menu = None;
            Some((name, poly))
        }
    }
}
