use alloc::arc::{Arc, Weak};
use alloc::boxed::Box;
use core::mem;
use core::ops::{Deref, DerefMut};
use fs::Resource;
use sync::WaitQueue;
use system::error::{Error, Result, EACCES, EEXIST, EINVAL, EPERM, EPIPE};
use system::scheme::Packet;

/// A supervisor resource.
///
/// Reading from it will simply read the relevant registers to the buffer (see `Packet`).
///
/// Writing will simply left shift EAX by one byte, and then OR it with the byte from the buffer,
/// effectively writing the buffer to the EAX register (truncating the additional bytes).
pub struct SupervisorResource {
    recv: Arc<WaitQueue<Packet>>,
    send: Weak<WaitQueue<Packet>>
}

impl SupervisorResource {
    /// Create a new supervisor resource, supervising some PID.
    pub fn new(pid: usize) -> Result<Box<SupervisorResource>> {
        let mut contexts = ::env().contexts.lock();
        let cur_pid = try!(contexts.current()).pid;
        let ctx = try!(contexts.find_mut(pid));
        if ctx.supervised {
            if ctx.ppid != cur_pid {
                // Access denied if not parent
                Err(Error::new(EACCES))
            } else {
                if ctx.supervised_resource.is_some() {
                    // File exists if another supervisor handle is already present
                    Err(Error::new(EEXIST))
                } else {
                    // The receiver (who waits) holds the strong end, the sender holds the weak end
                    let request = Arc::new(WaitQueue::new());
                    let response = Arc::new(WaitQueue::new());

                    ctx.supervised_resource = Some(box SupervisorResource {
                        recv: response.clone(),
                        send: Arc::downgrade(&request)
                    });

                    Ok(box SupervisorResource {
                        recv: request.clone(),
                        send: Arc::downgrade(&response)
                    })
                }
            }
        } else {
            // Operation not permitted if context is not supervisable
            Err(Error::new(EPERM))
        }
    }
}

impl Resource for SupervisorResource {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        if buf.len() == mem::size_of::<Packet>() {
            let packet = self.recv.receive();

            for (b, p) in buf.iter_mut().zip(packet.deref().iter()) {
                *b = *p;
            }

            Ok(mem::size_of::<Packet>())
        } else {
            // Packet not sized correctly, invalid argument
            Err(Error::new(EINVAL))
        }
    }

    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        if buf.len() == mem::size_of::<Packet>() {
            match self.send.upgrade() {
                Some(send) => {
                    let mut packet = Packet::default();

                    for (b, p) in buf.iter().zip(packet.deref_mut().iter_mut()) {
                        *p = *b
                    }

                    send.send(packet);

                    Ok(mem::size_of::<Packet>())
                },
                None => {
                    // Receiver disconnected, broken pipe
                    Err(Error::new(EPIPE))
                }
            }
        } else {
            // Packet not sized correctly, invalid argument
            Err(Error::new(EINVAL))
        }
    }
}
