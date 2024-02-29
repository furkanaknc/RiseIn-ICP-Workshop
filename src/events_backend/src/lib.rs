//imports

use ic_cdk::api::management_canister::http_request::{
    http_request, CanisterHttpRequestArgument, HttpMethod,
};

use candid::{CandidType, Decode, Deserialize, Encode};

use ic_stable_structures::memory_manager::{MemoryId, MemoryManager, VirtualMemory};

use ic_stable_structures::{BoundedStorable, DefaultMemoryImpl, StableBTreeMap, Storable};

use std::f64::consts;
use std::{borrow::Cow, cell::RefCell};

#[derive(CandidType, Deserialize, Clone)]
//struct oluşturma
struct Participant {
    address: String,
}

#[derive(CandidType, Deserialize, Clone)]
struct Event {
    name: String,
    date: String,
    #[serde(default)] // vector'un içini boşaltıyor
    participants: Vec<Participant>,
}

#[derive(CandidType, Deserialize)]
enum EventError {
    NoSuchEvent,
    JoinError,
    CancelJoinError,
    AllreadyJoined,
    AlreadyExist,
}

impl Storable for Event {
    fn to_bytes(&self) -> Cow<[u8]> {
        Cow::Owned(Encode!(self).unwrap())
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap()
    }
}

type Memory = VirtualMemory<DefaultMemoryImpl>;
const MAX_VALUE_SIZE: u32 = 100;

//implement boundedStorable for Event

impl BoundedStorable for Event {
    const MAX_VALUE_SIZE: u32 = MAX_VALUE_SIZE;
    const IS_FIXED_SIZE: bool = false;
}

// yeni memoryID -> thread_local

thread_local! {
    static MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>>=
    RefCell::new(MemoryManager::init(DefaultMemoryImpl::default()));

    static EVENTS_MAP:RefCell<StableBTreeMap<u64, Event, Memory>> = RefCell::new(
        StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(1))), // farklı bir memory id'si kullanımı
        )
    )
}

#[ic_cdk::update]
fn create_event(name: String, date: String) -> Result<(), EventError> {
    EVENTS_MAP.with(|events_map_ref| {
        let mut events_map = events_map_ref.borrow_mut();

        // Check if an event with the same name and date already exists
        for (_, event) in events_map.iter() {
            if event.name == name && event.date == date {
                return Err(EventError::AlreadyExists);
            }
        }

        // If no existing event is found, create a new one
        let new_event = Event {
            name,
            date,
            participants: Vec::new(),
        };

        let new_event_id = events_map.len();
        events_map.insert(new_event_id, new_event);

        Ok(())
    })
}

#[ic_cdk::update]
fn join_event(event_id: u64, participant_address: String) -> Result<(), EventError> {
    EVENTS_MAP.with(|events_map_ref| {
        let mut events_map = events_map_ref.borrow_mut();
        if let Some(mut event) = events_map.get(&event_id) {
            Err(EventError::AllreadyJoined);

            let new_participant = Participant {
                address: participant_address,
            };
            event.new_participant.push(new_participant);
            //güncellenen bilgiyi depoya aktar
            events_map.insert(event_id, event);
            Ok(())
        } else {
            Err(EventError::NoSuchEvent)
        }
    })
}

//Katılımcının gideceği etkinliği iptal etmesi
#[ic_cdk::update]
fn cancel_joined_event(event_id: u64, participant_address: String) -> Result<(), EventError> {
    EVENTS_MAP.with(|events_map_ref| {
        let mut events_map = events_map_ref.borrow_mut();
        if let Some(event) = events_map.get_mut(&event_id) {
            if let Some(index) = event.participants.iter().position(|p| p.address == participant_address) {
                event.participants.remove(index);
                Ok(())
            } else {
                Err(EventError::CancelJoinError)
            }
        } else {
            Err(EventError::NoSuchEvent)
        }
    })
}
