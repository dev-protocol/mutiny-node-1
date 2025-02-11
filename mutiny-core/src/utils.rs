use bitcoin::Network;
use core::cell::{RefCell, RefMut};
use core::ops::{Deref, DerefMut};
use core::time::Duration;
use lightning::routing::scoring::LockableScore;
use lightning::routing::scoring::Score;
use lightning::util::ser::Writeable;
use lightning::util::ser::Writer;

pub(crate) fn min_lightning_amount(network: Network) -> u64 {
    match network {
        Network::Bitcoin => 50_000,
        Network::Testnet | Network::Signet | Network::Regtest => 10_000,
    }
}

pub async fn sleep(millis: i32) {
    #[cfg(target_arch = "wasm32")]
    {
        let mut cb = |resolve: js_sys::Function, _reject: js_sys::Function| {
            web_sys::window()
                .unwrap()
                .set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, millis)
                .unwrap();
        };
        let p = js_sys::Promise::new(&mut cb);
        wasm_bindgen_futures::JsFuture::from(p).await.unwrap();
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::thread::sleep(Duration::from_millis(millis.try_into().unwrap()));
    }
}

pub fn now() -> Duration {
    #[cfg(target_arch = "wasm32")]
    return instant::SystemTime::now()
        .duration_since(instant::SystemTime::UNIX_EPOCH)
        .unwrap();

    #[cfg(not(target_arch = "wasm32"))]
    return std::time::SystemTime::now()
        .duration_since(std::time::SystemTime::UNIX_EPOCH)
        .unwrap();
}

pub type LockResult<Guard> = Result<Guard, ()>;

pub struct Mutex<T: ?Sized> {
    inner: RefCell<T>,
}

unsafe impl<T: ?Sized> Send for Mutex<T> {}
unsafe impl<T: ?Sized> Sync for Mutex<T> {}

#[must_use = "if unused the Mutex will immediately unlock"]
pub struct MutexGuard<'a, T: ?Sized + 'a> {
    lock: RefMut<'a, T>,
}

impl<T: ?Sized> Deref for MutexGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        self.lock.deref()
    }
}

impl<T: ?Sized> DerefMut for MutexGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        self.lock.deref_mut()
    }
}

impl<T> Mutex<T> {
    pub fn new(inner: T) -> Mutex<T> {
        Mutex {
            inner: RefCell::new(inner),
        }
    }

    pub fn lock(&self) -> LockResult<MutexGuard<'_, T>> {
        Ok(MutexGuard {
            lock: self.inner.borrow_mut(),
        })
    }
}

impl<'a, T: 'a + Score> LockableScore<'a> for Mutex<T> {
    type Locked = MutexGuard<'a, T>;

    fn lock(&'a self) -> MutexGuard<'a, T> {
        Mutex::lock(self).expect("Failed to lock mutex")
    }

    type Score = T;
}

impl<S: Writeable> Writeable for Mutex<S> {
    fn write<W: Writer>(&self, writer: &mut W) -> Result<(), lightning::io::Error> {
        self.lock()
            .expect("Failed to lock mutex for write")
            .write(writer)
    }
}

impl<'a, S: Writeable> Writeable for MutexGuard<'a, S> {
    fn write<W: Writer>(&self, writer: &mut W) -> Result<(), lightning::io::Error> {
        S::write(&**self, writer)
    }
}

pub fn spawn<F>(future: F)
where
    F: core::future::Future<Output = ()> + 'static,
{
    #[cfg(not(target_arch = "wasm32"))]
    {
        tokio::task::LocalSet::new().spawn_local(future);
    }
    #[cfg(target_arch = "wasm32")]
    {
        wasm_bindgen_futures::spawn_local(future);
    }
}
