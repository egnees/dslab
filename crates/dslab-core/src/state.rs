use std::cell::RefCell;
use std::collections::{BinaryHeap, HashMap, HashSet, VecDeque};
use std::rc::Rc;

use rand::distributions::uniform::{SampleRange, SampleUniform};
use rand::distributions::{Alphanumeric, DistString};
use rand::prelude::*;
use rand_pcg::Pcg64;

use crate::component::Id;
use crate::event::{Event, EventData, EventId};
use crate::log::log_incorrect_event;
use crate::{async_disabled, async_enabled};

async_enabled! {
    use std::any::TypeId;
    use std::sync::mpsc::Sender;

    use futures::Future;

    use crate::async_core::await_details::EventKey;
    use crate::async_core::awaiters::{Awaiter, AwaitersWithSourceStorage};
    use crate::async_core::shared_state::{AwaitEventSharedState, EmptyData, TimerFuture};
    use crate::async_core::shared_state::{AwaitKey, AwaitResultSetter};
    use crate::async_core::task::Task;
    use crate::async_core::timer::{Timer, TimerId};
}

/// Epsilon to compare floating point values for equality.
pub const EPSILON: f64 = 1e-12;

async_disabled! {
    #[derive(Clone)]
    pub struct SimulationState {
        clock: f64,
        rand: Pcg64,
        events: BinaryHeap<Event>,
        ordered_events: VecDeque<Event>,
        canceled_events: HashSet<EventId>,
        event_count: u64,

        name_to_id: HashMap<String, Id>,
        names: Rc<RefCell<Vec<String>>>,
    }
}

async_enabled! {
    type KeyGetterFunction = Rc<dyn Fn(&dyn EventData) -> EventKey>;

    #[derive(Clone)]
    pub struct SimulationState {
        clock: f64,
        rand: Pcg64,
        events: BinaryHeap<Event>,
        ordered_events: VecDeque<Event>,
        canceled_events: HashSet<EventId>,
        event_count: u64,

        name_to_id: HashMap<String, Id>,
        names: Rc<RefCell<Vec<String>>>,
        registered_handlers: Vec<bool>,

        awaiters: HashMap<AwaitKey, Awaiter>,
        awaiters_with_source: AwaitersWithSourceStorage,
        awaiters_capacity: usize,
        key_getters: HashMap<TypeId, KeyGetterFunction>,

        timers: BinaryHeap<Timer>,
        canceled_timers: HashSet<TimerId>,
        timer_count: u64,

        executor: Sender<Rc<Task>>,
    }
}

impl SimulationState {
    async_disabled! {
        pub fn new(seed: u64) -> Self {
            Self {
                clock: 0.0,
                rand: Pcg64::seed_from_u64(seed),
                events: BinaryHeap::new(),
                ordered_events: VecDeque::new(),
                canceled_events: HashSet::new(),
                event_count: 0,
                name_to_id: HashMap::new(),
                names: Rc::new(RefCell::new(Vec::new())),
            }
        }
    }
    async_enabled! {
        pub fn new(seed: u64, task_sender: Sender<Rc<Task>>) -> Self {
            Self {
                clock: 0.0,
                rand: Pcg64::seed_from_u64(seed),
                events: BinaryHeap::new(),
                ordered_events: VecDeque::new(),
                canceled_events: HashSet::new(),
                event_count: 0,
                name_to_id: HashMap::new(),
                names: Rc::new(RefCell::new(Vec::new())),
                // Async stuff
                registered_handlers: Vec::new(),
                awaiters: HashMap::new(),
                awaiters_with_source: AwaitersWithSourceStorage::new(),
                awaiters_capacity: 4,
                key_getters: HashMap::new(),
                timers: BinaryHeap::new(),
                canceled_timers: HashSet::new(),
                timer_count: 0,
                executor: task_sender,
            }
        }
    }

    pub fn get_names(&self) -> Rc<RefCell<Vec<String>>> {
        self.names.clone()
    }

    pub fn lookup_id(&self, name: &str) -> Id {
        *self.name_to_id.get(name).unwrap()
    }

    pub fn lookup_name(&self, id: Id) -> String {
        self.names.borrow()[id as usize].clone()
    }

    pub fn register(&mut self, name: &str) -> Id {
        if let Some(&id) = self.name_to_id.get(name) {
            return id;
        }
        let id = self.name_to_id.len() as Id;
        self.name_to_id.insert(name.to_owned(), id);
        self.names.borrow_mut().push(name.to_owned());
        self.on_register();
        id
    }

    pub fn time(&self) -> f64 {
        self.clock
    }

    pub fn set_time(&mut self, time: f64) {
        self.clock = time;
    }

    pub fn rand(&mut self) -> f64 {
        self.rand.gen_range(0.0..1.0)
    }

    pub fn gen_range<T, R>(&mut self, range: R) -> T
    where
        T: SampleUniform,
        R: SampleRange<T>,
    {
        self.rand.gen_range(range)
    }

    pub fn sample_from_distribution<T, Dist: Distribution<T>>(&mut self, dist: &Dist) -> T {
        dist.sample(&mut self.rand)
    }

    pub fn random_string(&mut self, len: usize) -> String {
        Alphanumeric.sample_string(&mut self.rand, len)
    }

    pub fn add_event<T>(&mut self, data: T, src: Id, dst: Id, delay: f64) -> EventId
    where
        T: EventData,
    {
        let event_id = self.event_count;
        let event = Event {
            id: event_id,
            time: self.clock + delay.max(0.),
            src,
            dst,
            data: Box::new(data),
        };
        if delay >= -EPSILON {
            self.events.push(event);
            self.event_count += 1;
            event_id
        } else {
            log_incorrect_event(event, &format!("negative delay {}", delay));
            panic!("Event delay is negative! It is not allowed to add events from the past.");
        }
    }

    pub fn add_ordered_event<T>(&mut self, data: T, src: Id, dst: Id, delay: f64) -> EventId
    where
        T: EventData,
    {
        if !self.can_add_ordered_event(delay) {
            panic!("Event order is broken! Ordered events should be added in non-decreasing order of their time.");
        }
        let last_time = self.ordered_events.back().map_or(f64::MIN, |x| x.time);
        let event_id = self.event_count;
        let event = Event {
            id: event_id,
            // max is used to enforce time order despite of floating-point errors
            time: last_time.max(self.clock + delay),
            src,
            dst,
            data: Box::new(data),
        };
        if delay >= 0. {
            self.ordered_events.push_back(event);
            self.event_count += 1;
            event_id
        } else {
            log_incorrect_event(event, &format!("negative delay {}", delay));
            panic!("Event delay is negative! It is not allowed to add events from the past.");
        }
    }

    pub fn can_add_ordered_event(&self, delay: f64) -> bool {
        if let Some(evt) = self.ordered_events.back() {
            // small epsilon is used to account for floating-point errors
            if delay + self.clock < evt.time - EPSILON {
                return false;
            }
        }
        true
    }

    pub fn next_event(&mut self) -> Option<Event> {
        loop {
            let maybe_heap = self.events.peek();
            let maybe_deque = self.ordered_events.front();
            if maybe_heap.is_some() && (maybe_deque.is_none() || maybe_heap.unwrap() > maybe_deque.unwrap()) {
                let event = self.events.pop().unwrap();
                if !self.canceled_events.remove(&event.id) {
                    self.clock = event.time;
                    return Some(event);
                }
            } else if maybe_deque.is_some() {
                let event = self.ordered_events.pop_front().unwrap();
                if !self.canceled_events.remove(&event.id) {
                    self.clock = event.time;
                    return Some(event);
                }
            } else {
                return None;
            }
        }
    }

    pub fn peek_event(&mut self) -> Option<&Event> {
        loop {
            let maybe_heap = self.events.peek();
            let maybe_deque = self.ordered_events.front();
            let heap_event_id = maybe_heap.map(|e| e.id).unwrap_or(0);
            let deque_event_id = maybe_deque.map(|e| e.id).unwrap_or(0);

            if maybe_heap.is_some() && (maybe_deque.is_none() || maybe_heap.unwrap() > maybe_deque.unwrap()) {
                if self.canceled_events.remove(&heap_event_id) {
                    self.events.pop().unwrap();
                } else {
                    return self.events.peek();
                }
            } else if maybe_deque.is_some() {
                if self.canceled_events.remove(&deque_event_id) {
                    self.ordered_events.pop_front().unwrap();
                } else {
                    return self.ordered_events.front();
                }
            } else {
                return None;
            }
        }
    }

    pub fn cancel_event(&mut self, id: EventId) {
        self.canceled_events.insert(id);
    }

    pub fn cancel_events<F>(&mut self, pred: F)
    where
        F: Fn(&Event) -> bool,
    {
        for event in self.events.iter() {
            if pred(event) {
                self.canceled_events.insert(event.id);
            }
        }
        for event in self.ordered_events.iter() {
            if pred(event) {
                self.canceled_events.insert(event.id);
            }
        }
    }

    pub fn cancel_and_get_events<F>(&mut self, pred: F) -> Vec<Event>
    where
        F: Fn(&Event) -> bool,
    {
        let mut events = Vec::new();
        for event in self.events.iter() {
            if pred(event) {
                self.canceled_events.insert(event.id);
                events.push(event.clone());
            }
        }
        for event in self.ordered_events.iter() {
            if pred(event) {
                self.canceled_events.insert(event.id);
                events.push(event.clone());
            }
        }
        events
    }

    /// This function does not check events from `ordered_events`.
    pub fn cancel_heap_events<F>(&mut self, pred: F)
    where
        F: Fn(&Event) -> bool,
    {
        for event in self.events.iter() {
            if pred(event) {
                self.canceled_events.insert(event.id);
            }
        }
    }

    pub fn event_count(&self) -> u64 {
        self.event_count
    }

    pub fn dump_events(&self) -> Vec<Event> {
        let mut output = Vec::new();
        for event in self.events.iter() {
            if !self.canceled_events.contains(&event.id) {
                output.push((*event).clone())
            }
        }
        for event in self.ordered_events.iter() {
            if !self.canceled_events.contains(&event.id) {
                output.push((*event).clone())
            }
        }
        output.sort();
        // Because the sorting order of events is inverted to be used with BinaryHeap
        output.reverse();
        output
    }

    async_disabled! {
        fn on_register(&mut self) {}
        pub fn on_handler_added(&mut self, _id: Id) {}
        pub fn on_handler_removed(&mut self, _id: Id) {}
    }

    async_enabled! {
        fn on_register(&mut self) {
            self.registered_handlers.push(false)
        }

        pub fn on_handler_added(&mut self, id: Id) {
            self.registered_handlers[id as usize] = true;
        }

        pub fn on_handler_removed(&mut self, id: Id) {
            self.registered_handlers[id as usize] = false;
        }

        fn has_registered_handler(&self, component_id: Id) -> bool {
            if let Some(flag) = self.registered_handlers.get(component_id as usize) {
                *flag
            } else {
                false
            }
        }

        pub fn cancel_component_timers(&mut self, component_id: Id) {
            for timer in self.timers.iter() {
                if timer.component_id == component_id {
                    self.canceled_timers.insert(timer.id);
                }
            }
        }

        pub fn peek_timer(&mut self) -> Option<&Timer> {
            loop {
                if let Some(timer) = self.timers.peek() {
                    if !self.canceled_timers.remove(&timer.id) {
                        return Some(timer);
                    }
                } else {
                    return None;
                }
            }
        }

        pub fn next_timer(&mut self) -> Option<Timer> {
            loop {
                if let Some(timer) = self.timers.pop() {
                    if !self.canceled_timers.remove(&timer.id) {
                        self.clock = timer.time;
                        return Some(timer);
                    }
                } else {
                    return None;
                }
            }
        }

        pub(crate) fn add_awaiter_handler(&mut self, key: AwaitKey, src_opt: Option<Id>, state: Rc<RefCell<dyn AwaitResultSetter>>) {
            if let Some(src) = src_opt {
                if let Some(awaiter) = self.awaiters.get(&key) {
                    if awaiter.is_shared() {
                        panic!("awaiter for key {:?} (without source) already exists", key);
                    } else {
                        self.awaiters.remove(&key);
                    }
                }
                if let Some(awaiter) = self.awaiters_with_source.insert(key, src, Awaiter { state }) {
                    if awaiter.is_shared() {
                        panic!("awaiter for key {:?} and source {} already exists", key, src);
                    }
                }
            } else {
                if let Some(src) = self.awaiters_with_source.has_any_shared_awaiter_on_key(&key) {
                    panic!("awaiter for key {:?} with source {} already exists", key, src);
                }
                if let Some(awaiter) = self.awaiters.insert(key, Awaiter { state }) {
                    if awaiter.is_shared() {
                        panic!("awaiter for key {:?} (without source) already exists", key);
                    }
                }
            }
        }

        pub(crate) fn has_handler_on_key(&mut self, src: &Id, key: &AwaitKey) -> bool {
            if let Some(awaiter) = self.awaiters.get(key) {
                if awaiter.is_shared() {
                    return true;
                } else {
                    self.awaiters.remove(key);
                }
            }
            self.awaiters_with_source.has_shared_awaiter_on_key_with_source(key, src)
        }

        pub(crate) fn set_event_for_await_key(&mut self, src: Id, key: &AwaitKey, event: Event) -> bool {
            // panics if there is no awaiter
            let awaiter = self.awaiters.remove(key).unwrap_or_else(|| self.awaiters_with_source.remove(key, &src).unwrap());
            assert!(awaiter.is_shared(), "internal error: trying to set event for awaiter that is not shared");

            awaiter.state.borrow_mut().set_ok_completed_with_event(event);
            true
        }

        pub fn spawn(&mut self, future: impl Future<Output = ()> + 'static) {
            Task::spawn(future, self.executor.clone());
        }

        pub fn spawn_component(&mut self, component_id: Id, future: impl Future<Output = ()>) {
            assert!(
                self.has_registered_handler(component_id),
                "Spawning component without registered event handler is not supported. \
                Register handler for component {} before spawning it (empty impl EventHandler is OK).",
                component_id,
            );
            Task::spawn(future, self.executor.clone());
        }

        pub fn wait_for(&mut self, component_id: Id, timeout: f64) -> TimerFuture {
            let state = Rc::new(RefCell::new(AwaitEventSharedState::<EmptyData>::new(component_id)));
            self.add_timer_on_state(component_id, timeout, state.clone());

            TimerFuture { state }
        }

        pub(crate) fn add_timer_on_state(
            &mut self,
            component_id: Id,
            timeout: f64,
            state: Rc<RefCell<dyn AwaitResultSetter>>,
        ) {
            self.timer_count += 1;
            let timer = Timer::new(self.timer_count, component_id, self.time() + timeout, state);
            self.timers.push(timer);
        }

        pub fn get_key_getter(&self, type_id: TypeId) -> Option<KeyGetterFunction> {
            self.key_getters.get(&type_id).cloned()
        }

        pub fn register_key_getter_for<T: EventData>(&mut self, key_getter: impl Fn(&T) -> EventKey + 'static) {
            self.key_getters.insert(TypeId::of::<T>(), Rc::new(move |raw_data| {
                if let Some(data) = raw_data.downcast_ref::<T>() {
                    key_getter(data)
                } else {
                    panic!(
                        "internal error: key getter for type {} is incorrectly used for type {}",
                        std::any::type_name::<T>(),
                        serde_type_name::type_name(&raw_data).unwrap(),
                    );
                }
            }));
        }

        pub fn cleanup_empty_awaiters(&mut self) {
            if self.awaiters.len() + self.awaiters_with_source.len() < self.awaiters_capacity {
                return;
            }
            self.awaiters.retain(|_key, awaiter| awaiter.is_shared());
            self.awaiters_with_source.remove_not_shared_awaiters();

            // Extend capacity if it is too small to keep all shared awaiters
            if self.awaiters.len() + self.awaiters_with_source.len() >= self.awaiters_capacity / 2 {
                self.awaiters_capacity *= 2;
            }
        }
    }
}
