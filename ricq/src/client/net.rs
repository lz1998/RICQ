use std::net::{Ipv4Addr, SocketAddr};
use std::sync::atomic::Ordering;
use std::sync::Arc;

use bytes::Bytes;
use futures::{SinkExt, StreamExt};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::sync::broadcast;
use tokio_util::codec::LengthDelimitedCodec;

use crate::client::NetworkStatus;

use super::Client;

pub type OutPktSender = broadcast::Sender<Bytes>;

impl crate::Client {
    pub fn get_address(&self) -> SocketAddr {
        // TODO 选择最快地址
        SocketAddr::new(Ipv4Addr::new(114, 221, 144, 215).into(), 80)
    }

    pub fn get_status(&self) -> u8 {
        self.status.load(Ordering::Relaxed)
    }

    /// 开始处理流数据
    ///
    ///**Notice: 该方法仅开始处理包，需要手动登录并开始心跳包**
    pub async fn start<S: AsyncRead + AsyncWrite>(self: &Arc<Self>, stream: S) {
        self.status
            .store(NetworkStatus::Running as u8, Ordering::Relaxed);
        self.net_loop(stream).await; // 阻塞到断开
        self.disconnect();
        if self.get_status() == (NetworkStatus::Running as u8) {
            self.status
                .store(NetworkStatus::NetworkOffline as u8, Ordering::Relaxed);
        }
    }

    pub fn stop(&self, status: NetworkStatus) {
        self.disconnect();
        self.status.store(status as u8, Ordering::Relaxed);
        self.online.store(false, Ordering::Relaxed);
    }

    fn disconnect(&self) {
        // TODO dispatch disconnect event
        // don't unwrap (Err means there is no receiver.)
        self.disconnect_signal.send(()).ok();
    }

    async fn net_loop<S: AsyncRead + AsyncWrite>(self: &Arc<Client>, stream: S) {
        let (mut write_half, mut read_half) = LengthDelimitedCodec::builder()
            .length_field_length(4)
            .length_adjustment(-4)
            .new_framed(stream)
            .split();
        let cli = self.clone();
        // 外发包 Channel Receiver
        let mut rx = self.out_pkt_sender.subscribe();
        let mut disconnect_signal = self.disconnect_signal.subscribe();
        loop {
            tokio::select! {
                input = read_half.next() => {
                    if let Some(Ok(mut input)) = input {
                        if let Ok(pkt) = cli.engine.read().await.transport.decode_packet(&mut input) {
                            cli.process_income_packet(pkt).await;
                        } else {
                            break;
                        }
                    } else {
                        break;
                    }
                }
                output = rx.recv() => {
                    if let Ok(output) = output {
                        if write_half.send(output).await.is_err() {
                            break;
                        }
                    }
                }
                _ = disconnect_signal.recv() => {
                    break;
                }
            }
        }
    }
}
