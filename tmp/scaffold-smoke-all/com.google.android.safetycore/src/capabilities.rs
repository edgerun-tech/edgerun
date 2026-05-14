#[derive(Debug, Default)]
pub struct Capabilities {
    pub network: NetworkCapability,
    pub location: LocationCapability,
    pub notifications: NotificationCapability,
    pub storage: StorageCapability,
    pub local_state: LocalStateCapability,
    pub camera_media: CameraMediaCapability,
    pub secrets_crypto: SecretsCryptoCapability,
}

#[derive(Debug, Default)] pub struct NetworkCapability;
#[derive(Debug, Default)] pub struct LocationCapability;
#[derive(Debug, Default)] pub struct NotificationCapability;
#[derive(Debug, Default)] pub struct StorageCapability;
#[derive(Debug, Default)] pub struct LocalStateCapability;
#[derive(Debug, Default)] pub struct CameraMediaCapability;
#[derive(Debug, Default)] pub struct BluetoothNearbyCapability;
#[derive(Debug, Default)] pub struct SecretsCryptoCapability;
