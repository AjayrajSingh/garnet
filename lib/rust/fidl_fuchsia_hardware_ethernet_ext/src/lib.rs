// Copyright 2018 The Fuchsia Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use fidl_fuchsia_hardware_ethernet as fidl;

use bitflags::bitflags;

#[derive(PartialEq, Eq, Debug, Clone, Copy, Hash)]
pub struct MacAddress {
    pub octets: [u8; 6],
}

impl From<fidl::MacAddress> for MacAddress {
    fn from(fidl::MacAddress { octets }: fidl::MacAddress) -> Self {
        Self { octets }
    }
}

impl Into<fidl::MacAddress> for MacAddress {
    fn into(self) -> fidl::MacAddress {
        let Self { octets } = self;
        fidl::MacAddress { octets }
    }
}

impl std::fmt::Display for MacAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let Self { octets } = self;
        for (i, byte) in octets.iter().enumerate() {
            if i > 0 {
                write!(f, ":")?;
            }
            write!(f, "{:02x}", byte)?;
        }
        Ok(())
    }
}

impl serde::Serialize for MacAddress {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(&self)
    }
}

impl<'de> serde::Deserialize<'de> for MacAddress {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = <String as serde::Deserialize>::deserialize(deserializer)?;
        <Self as std::str::FromStr>::from_str(&s).map_err(serde::de::Error::custom)
    }
}

impl std::str::FromStr for MacAddress {
    type Err = failure::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use failure::ResultExt;

        let mut octets = [0; 6];
        let mut iter = s.split(':');
        for (i, octet) in octets.iter_mut().enumerate() {
            let next_octet = iter.next().ok_or_else(|| {
                failure::format_err!("MAC address [{}] only specifies {} out of 6 octets", s, i)
            })?;
            *octet = u8::from_str_radix(next_octet, 16)
                .with_context(|_| format!("could not parse hex integer from {}", next_octet))?;
        }
        if iter.next().is_some() {
            return Err(failure::format_err!("MAC address has more than six octets: {}", s));
        }
        Ok(MacAddress { octets })
    }
}

bitflags! {
    /// Features supported by an Ethernet device.
    #[repr(transparent)]
    pub struct EthernetFeatures: u32 {
        /// The Ethernet device is a wireless device.
        const WLAN = fidl::INFO_FEATURE_WLAN;
        /// The Ethernet device does not represent a hardware device.
        const SYNTHETIC = fidl::INFO_FEATURE_SYNTH;
        /// The Ethernet device is a loopback device.
        ///
        /// This bit should not be set outside of network stacks.
        const LOOPBACK = fidl::INFO_FEATURE_LOOPBACK;
    }
}

impl EthernetFeatures {
    pub fn is_physical(&self) -> bool {
        !self.intersects(Self::SYNTHETIC | Self::LOOPBACK)
    }
}

impl std::str::FromStr for EthernetFeatures {
    type Err = failure::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.as_ref() {
            "synthetic" => Ok(Self::SYNTHETIC),
            "loopback" => Ok(Self::LOOPBACK),
            "wireless" => Ok(Self::WLAN),
            s => Err(failure::format_err!("unknown network interface feature \"{}\"", s)),
        }
    }
}

/// Information retrieved about an Ethernet device.
#[derive(Debug)]
pub struct EthernetInfo {
    /// The features supported by the device.
    pub features: EthernetFeatures,
    /// The maximum transmission unit (MTU) of the device.
    pub mtu: u32,
    /// The MAC address of the device.
    pub mac: MacAddress,
}

impl From<fidl::Info> for EthernetInfo {
    fn from(fidl::Info { features, mtu, mac }: fidl::Info) -> Self {
        let features = EthernetFeatures::from_bits_truncate(features);
        let mac = mac.into();
        Self { features, mtu, mac }
    }
}

bitflags! {
    /// Status flags for an Ethernet device.
    #[repr(transparent)]
    pub struct EthernetStatus: u32 {
        /// The Ethernet device is online, meaning its physical link is up.
        const ONLINE = fidl::DEVICE_STATUS_ONLINE;
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use std::collections::HashMap;
    use std::str::FromStr;

    #[test]
    fn mac_addr_from_str_with_valid_str_returns_mac_addr() {
        let result = MacAddress::from_str("AA:BB:CC:DD:EE:FF").unwrap();
        let expected = MacAddress { octets: [0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF] };

        assert_eq!(expected, result);
    }

    #[test]
    fn mac_addr_from_str_with_invalid_digit_returns_err() {
        let result = MacAddress::from_str("11:22:33:44:55:GG");

        assert!(result.is_err());
    }

    #[test]
    fn mac_addr_from_str_with_invalid_format_returns_err() {
        let result = MacAddress::from_str("11-22-33-44-55-66");

        assert!(result.is_err());
    }

    #[test]
    fn mac_addr_from_str_with_empty_string_returns_err() {
        let result = MacAddress::from_str("");

        assert!(result.is_err());
    }

    #[test]
    fn mac_addr_from_str_with_extra_quotes_returns_err() {
        let result = MacAddress::from_str("\"11:22:33:44:55:66\"");

        assert!(result.is_err());
    }

    #[test]
    fn valid_mac_addr_array_deserializes_to_vec_of_mac_addrs() {
        let result: Vec<MacAddress> =
            serde_json::from_str("[\"11:11:11:11:11:11\", \"AA:AA:AA:AA:AA:AA\"]").unwrap();
        let expected = vec![
            MacAddress { octets: [0x11, 0x11, 0x11, 0x11, 0x11, 0x11] },
            MacAddress { octets: [0xAA, 0xAA, 0xAA, 0xAA, 0xAA, 0xAA] },
        ];

        assert_eq!(expected, result);
    }

    #[test]
    fn mac_addr_to_mac_addr_map_deserializes_to_hashmap() {
        let result: HashMap<MacAddress, MacAddress> =
            serde_json::from_str("{\"11:22:33:44:55:66\": \"AA:BB:CC:DD:EE:FF\"}").unwrap();
        let mut expected = HashMap::new();
        expected.insert(
            MacAddress { octets: [0x11, 0x22, 0x33, 0x44, 0x55, 0x66] },
            MacAddress { octets: [0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF] },
        );

        assert_eq!(expected, result);
    }

    #[test]
    fn mac_addr_to_mac_addr_map_serializes_to_valid_json() {
        let mut mac_addr_map = HashMap::new();
        mac_addr_map.insert(
            MacAddress { octets: [0x11, 0x22, 0x33, 0x44, 0x55, 0x66] },
            MacAddress { octets: [0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF] },
        );

        let result = serde_json::to_string(&mac_addr_map).unwrap();

        assert_eq!("{\"11:22:33:44:55:66\":\"aa:bb:cc:dd:ee:ff\"}", result);
    }
}
