use std::sync::{mpsc, atomic::Ordering};
use std::thread;
use std::time::{Duration,Instant};
use super::*;

#[test]
fn cache_only_read_and_unrelated_remote_read_do_not_wait_for_network() {
    let (entered_tx,entered_rx)=mpsc::channel();
    let (release_tx,release_rx)=mpsc::channel(); let release=Mutex::new(release_rx);
    let h=Harness::new(move|_,id| {
        if id==ID { entered_tx.send(()).unwrap(); release.lock().unwrap().recv().unwrap(); }
        sketch(5)
    });
    h.seed(URL,ID,2);
    let service=h.service.clone(); let pending=thread::spawn(move||service.read_remote(URL));
    entered_rx.recv_timeout(Duration::from_secs(2)).unwrap();
    let service=h.service.clone(); let (read_tx,read_rx)=mpsc::channel();
    let reader=thread::spawn(move|| { read_tx.send(service.read_cached(URL)).unwrap(); });
    let cached=read_rx.recv_timeout(Duration::from_secs(2));
    let service=h.service.clone(); let (other_tx,other_rx)=mpsc::channel();
    let other=thread::spawn(move|| {other_tx.send(service.read_remote(OTHER)).unwrap();});
    let other_reply=other_rx.recv_timeout(Duration::from_secs(2));
    release_tx.send(()).unwrap();
    reader.join().unwrap(); other.join().unwrap(); pending.join().unwrap();
    assert_eq!(cached.unwrap().version,Some(2));
    assert_eq!(other_reply.unwrap().version,Some(5));
}

#[test]
fn one_in_flight_read_is_shared_but_every_original_url_alias_is_cached() {
    let (entered_tx,entered_rx)=mpsc::channel();
    let (release_tx,release_rx)=mpsc::channel(); let release=Mutex::new(release_rx);
    let h=Harness::new(move|_,_| {entered_tx.send(()).unwrap(); release.lock().unwrap().recv().unwrap(); sketch(8)});
    let service=h.service.clone(); let first=thread::spawn(move||service.read_remote(URL));
    entered_rx.recv_timeout(Duration::from_secs(2)).unwrap();
    let service=h.service.clone(); let second=thread::spawn(move||service.read_remote(ALIAS));
    let until=Instant::now()+Duration::from_secs(2);
    let joined=loop {
        let joined=h.service.state.lock().unwrap().flights.values().any(|f|Arc::strong_count(f)>=3);
        if joined || Instant::now()>=until {break joined;}
        thread::yield_now();
    };
    release_tx.send(()).unwrap();
    assert_eq!(first.join().unwrap().version,Some(8));
    assert_eq!(second.join().unwrap().version,Some(8));
    assert!(joined,"second caller joined the active flight");
    assert_eq!(h.fetch.calls.load(Ordering::SeqCst),1);
    assert!(h.cache.disk.read(URL,SCOPE).unwrap().is_some());
    assert!(h.cache.disk.read(ALIAS,SCOPE).unwrap().is_some());
}

#[test]
fn a_late_success_or_401_from_old_credentials_cannot_change_the_new_scope() {
    for late_failure in [false,true] {
        let (entered_tx,entered_rx)=mpsc::channel();
        let (release_tx,release_rx)=mpsc::channel(); let release=Mutex::new(release_rx);
        let h=Harness::new(move|credential,_| {
            if credential=="secret-a" {
                entered_tx.send(()).unwrap(); release.lock().unwrap().recv().unwrap();
                if late_failure {error("unauthorized")} else {sketch(90)}
            } else {sketch(1)}
        });
        let service=h.service.clone(); let old=thread::spawn(move||service.read_remote(URL));
        entered_rx.recv_timeout(Duration::from_secs(2)).unwrap();
        h.service.set_pat("new-credential").unwrap();
        let new=h.service.read_remote(URL);
        release_tx.send(()).unwrap();
        assert_eq!(old.join().unwrap().status,"superseded");
        assert_eq!(h.service.read_cached(URL).identity,new.identity);
        assert_eq!(h.service.read_cached(URL).version,Some(1));
        assert!(!h.credentials.current.lock().unwrap().as_ref().unwrap().revoked);
        assert_eq!(h.events.lock().unwrap().len(),1);
    }
}

#[test]
fn a_global_401_discards_another_diagrams_pending_success() {
    let (entered_tx,entered_rx)=mpsc::channel();
    let (release_tx,release_rx)=mpsc::channel(); let release=Mutex::new(release_rx);
    let h=Harness::new(move|_,id| {
        if id==ID {entered_tx.send(()).unwrap(); release.lock().unwrap().recv().unwrap(); sketch(8)}
        else {error("unauthorized")}
    });
    let service=h.service.clone(); let old=thread::spawn(move||service.read_remote(URL));
    entered_rx.recv_timeout(Duration::from_secs(2)).unwrap();
    assert_eq!(h.service.read_remote(OTHER).status,"unauthorized");
    release_tx.send(()).unwrap();
    assert_eq!(old.join().unwrap().status,"superseded");
    assert!(!h.cache.disk.path(URL).exists());
}

#[test]
fn credential_change_cannot_clear_between_commit_check_and_store() {
    let h=Harness::new(|_,_|sketch(5));
    let (entered_tx,entered_rx)=mpsc::channel();
    let (release_tx,release_rx)=mpsc::channel(); let release=Mutex::new(release_rx);
    *h.cache.on_store.lock().unwrap()=Some(Arc::new(move|| {
        entered_tx.send(()).unwrap(); release.lock().unwrap().recv().unwrap();
    }));
    let service=h.service.clone(); let read=thread::spawn(move||service.read_remote(URL));
    entered_rx.recv_timeout(Duration::from_secs(2)).unwrap();
    let (start_tx,start_rx)=mpsc::channel(); let (done_tx,done_rx)=mpsc::channel();
    let service=h.service.clone(); let change=thread::spawn(move|| {
        start_tx.send(()).unwrap(); service.set_pat("new").unwrap(); done_tx.send(()).unwrap();
    });
    start_rx.recv_timeout(Duration::from_secs(2)).unwrap();
    let early_change=done_rx.recv_timeout(Duration::from_millis(50));
    release_tx.send(()).unwrap();
    read.join().unwrap(); change.join().unwrap();
    assert!(early_change.is_err(),"credential invalidation must serialize with the store");
    assert!(!h.cache.disk.path(URL).exists(),"new credential change clears the committed old record");
    assert_eq!(h.service.read_cached(URL).status,"cache_miss");
}
