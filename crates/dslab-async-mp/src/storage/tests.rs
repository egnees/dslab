use std::{cell::RefCell, rc::Rc};

use rand::seq::SliceRandom;
use rand::thread_rng;
use rand_pcg::Pcg64;
use rand_seeder::{Seeder, SipHasher};

use dslab_core::{async_core::EventKey, Event, EventHandler, Simulation, SimulationContext};
use dslab_storage::{
    disk::{self, DiskBuilder},
    events::{DataReadCompleted, DataReadFailed, DataWriteCompleted, DataWriteFailed},
};

use crate::storage::result::StorageError;

use super::{file::File, file_manager::FileManager, model::ModelWrapper, register::register_key_getters};

/// Represents stub for file manager.
impl EventHandler for FileManager {
    fn on(&mut self, _event: Event) {
        // do nothing
    }
}

/// Build simulation with [`FileHolder`] and returns [`FileHolder`] context.
fn build_simulation() -> (Simulation, Rc<RefCell<FileManager>>) {
    let mut sim = Simulation::new(12345);
    let disk_ctx = sim.create_context("disk");
    let disk = Rc::new(RefCell::new(
        DiskBuilder::simple(1 << 20, 1024., 1024.).build(disk_ctx.clone()),
    ));
    sim.add_handler("disk", disk.clone());

    let file_manager_ctx = sim.create_context("file_manager");
    let file_manager = Rc::new(RefCell::new(FileManager::new(disk, disk_ctx, file_manager_ctx)));
    sim.add_handler("file_manager", file_manager.clone());

    register_key_getters(&mut sim);

    (sim, file_manager)
}

#[test]
fn file_works() {
    let (mut sim, file_manager) = build_simulation();

    // Append to file.
    let mut file = file_manager.borrow_mut().create_file("file").unwrap();
    let test_ctx = file_manager.borrow().ctx.clone();
    test_ctx.spawn(async move {
        let data_to_append = "append".as_bytes();
        let appended_bytes = file.append(data_to_append).await.unwrap();
        assert_eq!(appended_bytes, data_to_append.len() as u64);
    });
    sim.step_until_no_events();

    // Read from file.
    file = file_manager.borrow_mut().open_file("file").unwrap();
    test_ctx.spawn(async move {
        let mut content = vec![0u8; 1024];
        let read_bytes = file.read(0, &mut content).await.unwrap();
        assert_eq!(read_bytes, "append".len() as u64);
        assert_eq!(content[.."append".len()], "append".as_bytes().to_vec());
    });
    sim.step_until_no_events();

    // Crash storage while appending to file.
    file = file_manager.borrow_mut().open_file("file").unwrap();
    test_ctx.spawn(async move {
        let just_data = "just_data".as_bytes();
        let append_result = file.append(just_data).await;
        assert_eq!(append_result, Err(StorageError::Unavailable));
    });
    sim.step(); // step for initiating append
    file_manager.borrow_mut().crash_storage();
    sim.step_until_no_events();
}

#[test]
fn file_manager_works() {
    let (_, file_manager) = build_simulation();
    assert!(file_manager.borrow_mut().create_file("file1").is_ok());
    assert!(file_manager.borrow_mut().open_file("file1").is_ok());
    assert!(file_manager.borrow_mut().create_file("file1").is_err());

    file_manager.borrow_mut().crash_storage();

    assert!(file_manager.borrow_mut().open_file("file1").is_err());

    file_manager.borrow_mut().recover_storage();

    assert!(file_manager.borrow_mut().open_file("file1").is_err());
    assert!(file_manager.borrow_mut().create_file("file1").is_ok());
}

#[test]
fn concurrent_file_access() {
    let (mut sim, file_manager) = build_simulation();
    let files: Vec<String> = (1..=5).map(|n| format!("file_{}", n)).collect();
    for file in files.iter() {
        file_manager.borrow_mut().create_file(file).unwrap();
    }

    let test_ctx = file_manager.borrow().ctx.clone();
    for user in 0..125 {
        let file_names = files.clone();
        let manager = file_manager.clone();
        test_ctx.spawn(async move {
            let mut files = file_names
                .into_iter()
                .map(|name| manager.borrow_mut().open_file(&name).unwrap())
                .collect::<Vec<File>>();
            files.shuffle(&mut Seeder::from(user).make_rng::<Pcg64>());
            let user = format!("user_{}", user);
            for mut file in files.into_iter() {
                let content = format!("content_from_{}\n", user);
                let append_bytes = file.append(content.as_bytes()).await.unwrap();
                assert_eq!(append_bytes, content.len() as u64);
            }
        });
    }

    sim.step_until_no_events();

    test_ctx.spawn(async move {
        let files = files
            .into_iter()
            .map(|name| file_manager.borrow_mut().open_file(&name).unwrap())
            .collect::<Vec<File>>();

        for mut file in files.into_iter() {
            let mut buf = vec![0u8; 1024 * 16];
            let bytes = file.read(0, &mut buf).await.unwrap();
            for line in buf.as_slice()[..bytes as usize]
                .split(|c| *c == b'\n')
                .map(|token| String::from_utf8_lossy(token))
                .filter(|line| !line.is_empty())
            {
                let mut is_ok = false;
                for user in 0..125 {
                    if line == format!("content_from_user_{}", user) {
                        is_ok = true;
                        break;
                    }
                }
                assert!(is_ok);
            }
        }
    });

    sim.step_until_no_events();
}
