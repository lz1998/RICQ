use std::sync::Arc;

use ricq_core::jce;

use crate::client::event::MSFOfflineEvent;
use crate::client::{Client, NetworkStatus};
use crate::handler::QEvent;

impl Client {
    // TODO 待测试
    pub(crate) async fn process_msf_force_offline(
        self: &Arc<Self>,
        offline: jce::RequestMSFForceOffline,
    ) {
        self.send_msg_offline_rsp(offline.uin, offline.seq_no)
            .await
            .ok();
        self.stop(NetworkStatus::MsfOffline);
        self.handler
            .handle(QEvent::MSFOffline(MSFOfflineEvent {
                client: self.clone(),
                inner: offline,
            }))
            .await;
    }
}
