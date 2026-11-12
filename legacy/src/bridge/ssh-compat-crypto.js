// Replacement for the `crypto` namespace of the proprietary native
// @termius/libtermius module. Only the symmetric "FromEncryptionKey" system is
// needed by an offline profile: the background process encrypts secure fields
// of its IndexedDB records with a random local key. Its envelope is
//   [version=4][flags=1][24-byte nonce][XSalsa20-Poly1305 box]
// which is what the recovered bundle checks before decrypting (see `Yst`).
// Team/cloud cryptosystems are intentionally unsupported in this build.
import nacl from 'tweetnacl';
import { Buffer } from 'buffer';

const VERSION = 4;
const FLAGS = 1;
const HEADER = 2 + nacl.secretbox.nonceLength;

export const errorCodes = Object.freeze({
  'crypto::Error::None': 0,
  'crypto::Error::UnknownVersion': 1,
  'crypto::Error::UnsupportedVersion': 2,
  'crypto::Error::RNCryptorVersion': 3,
  'crypto::Error::IncorrectSecretKeySize': 4,
  'crypto::Error::IncorrectPublicKeySize': 5,
  'crypto::Error::IncorrectPrivateKeySize': 6,
  'crypto::Error::IncorrectSaltSize': 7,
  'crypto::Error::Unknown': 8,
  'crypto::Error::DecryptionFailed': 9,
});

class SecretBoxSystem {
  constructor(key) {
    this.lastError = errorCodes['crypto::Error::None'];
    this.key = key?.length === nacl.secretbox.keyLength ? new Uint8Array(key) : null;
    if (!this.key) this.lastError = errorCodes['crypto::Error::IncorrectSecretKeySize'];
  }
  getLastError() { return this.lastError; }
  encrypt(data) {
    if (!this.key) return null;
    const nonce = nacl.randomBytes(nacl.secretbox.nonceLength);
    const box = nacl.secretbox(new Uint8Array(Buffer.from(data)), nonce, this.key);
    this.lastError = errorCodes['crypto::Error::None'];
    return Buffer.concat([Buffer.from([VERSION, FLAGS]), Buffer.from(nonce), Buffer.from(box)]);
  }
  decrypt(data) {
    if (!this.key) return null;
    const input = Buffer.from(data);
    if (input.length < HEADER + nacl.secretbox.overheadLength) {
      this.lastError = errorCodes['crypto::Error::Unknown'];
      return null;
    }
    if (input[0] !== VERSION) {
      this.lastError = errorCodes['crypto::Error::UnsupportedVersion'];
      return null;
    }
    const opened = nacl.secretbox.open(
      new Uint8Array(input.subarray(HEADER)),
      new Uint8Array(input.subarray(2, HEADER)),
      this.key,
    );
    if (!opened) {
      this.lastError = errorCodes['crypto::Error::DecryptionFailed'];
      return null;
    }
    this.lastError = errorCodes['crypto::Error::None'];
    return Buffer.from(opened);
  }
}

class UnsupportedSystem {
  getLastError() { return errorCodes['crypto::Error::Unknown']; }
  encrypt() { return null; }
  decrypt() { return null; }
  exchangeSecret() { return null; }
}

export const crypto = {
  errorCodes,
  init() {},
  systems: {
    FromEncryptionKey: key => new SecretBoxSystem(key),
    FromKeyPair: () => new UnsupportedSystem(),
    FromPassword: () => new UnsupportedSystem(),
    ForOwner: () => new UnsupportedSystem(),
    ForMember: () => new UnsupportedSystem(),
  },
  utils: {
    generateEncryptionKey: () => Buffer.from(nacl.randomBytes(nacl.secretbox.keyLength)),
    generateKeyPair() {
      const pair = nacl.box.keyPair();
      return {publicKey:Buffer.from(pair.publicKey), privateKey:Buffer.from(pair.secretKey)};
    },
  },
};
