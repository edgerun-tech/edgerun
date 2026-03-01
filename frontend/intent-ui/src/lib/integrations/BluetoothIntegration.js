class BluetoothIntegration {
  constructor({
    integrationId = "bluetooth",
    optionalServices = []
  } = {}) {
    this.integrationId = String(integrationId || "bluetooth").trim();
    this.optionalServices = Array.from(new Set(
      (Array.isArray(optionalServices) ? optionalServices : [])
        .map((value) => String(value || "").trim())
        .filter(Boolean)
    ));
  }

  requireBluetooth() {
    if (typeof window === "undefined" || !window.isSecureContext) {
      throw new Error("Web Bluetooth requires a secure browser context (HTTPS).");
    }
    if (!navigator?.bluetooth?.requestDevice) {
      throw new Error("Web Bluetooth API is unavailable in this browser.");
    }
    return navigator.bluetooth;
  }

  isPermissionError(error) {
    const message = String(error instanceof Error ? error.message : error || "").toLowerCase();
    return message.includes("origin is not allowed") || message.includes("optionalservices");
  }

  isDisconnectedError(error) {
    const message = String(error instanceof Error ? error.message : error || "").toLowerCase();
    return message.includes("gatt server is disconnected") || message.includes("not connected");
  }

  buildRequestOptions({
    acceptAllDevices = true,
    filters = [],
    optionalServices = []
  } = {}) {
    const mergedOptionalServices = Array.from(new Set(
      [...this.optionalServices, ...(Array.isArray(optionalServices) ? optionalServices : [])]
        .map((value) => String(value || "").trim())
        .filter(Boolean)
    ));
    const options = {
      optionalServices: mergedOptionalServices
    };
    if (Array.isArray(filters) && filters.length > 0) {
      options.filters = filters;
    } else {
      options.acceptAllDevices = Boolean(acceptAllDevices);
    }
    return options;
  }

  async requestDevice(requestOptions = {}) {
    const bluetooth = this.requireBluetooth();
    return bluetooth.requestDevice(this.buildRequestOptions(requestOptions));
  }

  async getKnownDevices() {
    const bluetooth = this.requireBluetooth();
    if (typeof bluetooth.getDevices !== "function") return [];
    const devices = await bluetooth.getDevices();
    return Array.isArray(devices) ? devices : [];
  }

  async resolveDevice(preferredId = "", requestOptions = {}) {
    const normalizedId = String(preferredId || "").trim();
    if (normalizedId) {
      const known = await this.getKnownDevices();
      const match = known.find((device) => String(device?.id || "").trim() === normalizedId);
      if (match) return match;
    }
    return this.requestDevice(requestOptions);
  }

  async connectGatt(device) {
    if (!device?.gatt) throw new Error("Selected device does not expose GATT.");
    if (device.gatt.connected) return device.gatt;
    return device.gatt.connect();
  }

  async reconnectGatt(device) {
    if (!device?.gatt) throw new Error("Selected device does not expose GATT.");
    try {
      if (typeof device.gatt.disconnect === "function" && device.gatt.connected) {
        device.gatt.disconnect();
      }
    } catch {
      // best effort
    }
    return device.gatt.connect();
  }

  async getService(device, serviceUuid) {
    const serviceId = String(serviceUuid || "").trim();
    if (!serviceId) throw new Error("Missing service UUID.");

    let currentDevice = device;
    let server = await this.connectGatt(currentDevice);
    try {
      const service = await server.getPrimaryService(serviceId);
      return { device: currentDevice, server, service };
    } catch (error) {
      if (this.isDisconnectedError(error)) {
        server = await this.reconnectGatt(currentDevice);
        const service = await server.getPrimaryService(serviceId);
        return { device: currentDevice, server, service };
      }
      throw error;
    }
  }

  async getCharacteristic(device, serviceUuid, characteristicUuid) {
    let { device: resolvedDevice, server, service } = await this.getService(device, serviceUuid);
    try {
      const characteristic = await service.getCharacteristic(characteristicUuid);
      return { device: resolvedDevice, server, service, characteristic };
    } catch (error) {
      if (!this.isDisconnectedError(error)) throw error;
      server = await this.reconnectGatt(resolvedDevice);
      service = await server.getPrimaryService(serviceUuid);
      const characteristic = await service.getCharacteristic(characteristicUuid);
      return { device: resolvedDevice, server, service, characteristic };
    }
  }
}

export {
  BluetoothIntegration
};
