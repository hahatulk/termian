// Key helpers of @termius/libtermius. The renderer calls most of them
// synchronously while rendering (key type badges, form validation), so they
// parse the key formats directly: OpenSSH keys keep their public key blob
// unencrypted, PEM and PuTTY keys announce type and encryption in headers.
import { Buffer } from 'buffer';

const PEM = /-----BEGIN ([A-Z0-9 ]+)-----([\s\S]*?)-----END \1-----/;
const PUBLIC_LINE = /^((?:ssh|ecdsa|sk)-[A-Za-z0-9@.-]+)\s+([A-Za-z0-9+/]+={0,3})(?:\s+(.*))?$/;

function readString(buffer, offset) {
  if (offset + 4 > buffer.length) return null;
  const length = buffer.readUInt32BE(offset);
  const end = offset + 4 + length;
  if (end > buffer.length) return null;
  return {value: buffer.subarray(offset + 4, end), next: end};
}

// openssh-key-v1: magic, ciphername, kdfname, kdfoptions, nkeys, pubkey blobs.
function parseOpenSsh(body) {
  const data = Buffer.from(body.replace(/\s+/g, ''), 'base64');
  const magic = 'openssh-key-v1\0';
  if (data.subarray(0, magic.length).toString('latin1') !== magic) return null;
  const cipher = readString(data, magic.length);
  const kdf = cipher && readString(data, cipher.next);
  const kdfOptions = kdf && readString(data, kdf.next);
  if (!kdfOptions || kdfOptions.next + 4 > data.length) return null;
  const count = data.readUInt32BE(kdfOptions.next);
  const blob = count >= 1 ? readString(data, kdfOptions.next + 4) : null;
  const keyType = blob && readString(blob.value, 0);
  if (!keyType) return null;
  return {
    cipher: cipher.value.toString('latin1'),
    keyType: keyType.value.toString('latin1'),
    publicBlob: blob.value,
  };
}

function libtermiusType(sshType) {
  if (!sshType) return 'unknown';
  if (sshType.startsWith('sk-ssh-ed25519')) return 'sk-ed';
  if (sshType.startsWith('sk-ecdsa')) return 'sk-ec';
  if (sshType.startsWith('ssh-ed25519')) return 'ed';
  if (sshType.startsWith('ecdsa-')) return 'ec';
  if (sshType.startsWith('ssh-rsa') || sshType.startsWith('rsa-')) return 'rsa';
  if (sshType.startsWith('ssh-dss')) return 'dsa';
  return 'unknown';
}

function describePrivateKey(text) {
  const key = String(text ?? '').trim();
  if (key.startsWith('PuTTY-User-Key-File-')) {
    const type = /^PuTTY-User-Key-File-\d+:\s*(\S+)/.exec(key)?.[1];
    const encryption = /^Encryption:\s*(\S+)/m.exec(key)?.[1];
    return {valid: !!type, type: libtermiusType(type), encrypted: !!encryption && encryption !== 'none'};
  }
  const pem = PEM.exec(key);
  if (!pem) return {valid: false, type: 'unknown', encrypted: false};
  const label = pem[1];
  if (label === 'OPENSSH PRIVATE KEY') {
    const parsed = parseOpenSsh(pem[2]);
    if (!parsed) return {valid: false, type: 'unknown', encrypted: false};
    return {valid: true, type: libtermiusType(parsed.keyType), encrypted: parsed.cipher !== 'none', parsed};
  }
  const encrypted = /Proc-Type:\s*4,ENCRYPTED/.test(pem[2]) || label === 'ENCRYPTED PRIVATE KEY';
  const type = label.startsWith('RSA') ? 'rsa' : label.startsWith('EC') ? 'ec' : label.startsWith('DSA') ? 'dsa' : 'unknown';
  return {valid: /PRIVATE KEY$/.test(label), type, encrypted};
}

function parsePublicKey(text) {
  const match = PUBLIC_LINE.exec(String(text ?? '').trim());
  if (!match) return null;
  const blob = Buffer.from(match[2], 'base64');
  const embedded = readString(blob, 0)?.value.toString('latin1');
  if (embedded !== match[1]) return null;
  return {type: match[1], data: match[2], comment: match[3] ?? ''};
}

export function createKeyHelpers(invoke) {
  return {
    validatePrivateKeyFormat: key => describePrivateKey(key).valid,
    getKeyTypeFromPrivateKey: key => describePrivateKey(key).type,
    checkPrivateKeyEncrypted: key => describePrivateKey(key).encrypted,
    // A wrong passphrase is reported by the server-side auth attempt instead.
    checkPrivateKeyEncryptedWithPassword: () => true,
    validatePublicKeyFormat: key => parsePublicKey(key) != null,
    checkPublicKeyIsCertificate: key => parsePublicKey(key)?.type.includes('-cert-v01@openssh.com') ?? false,
    generateSshKeyFromPublicKey(key) {
      const parsed = parsePublicKey(key);
      return parsed ? {publicKey: `${parsed.type} ${parsed.data}`, privateKey: ''} : null;
    },
    generatePublicKeyFromPrivateKey(key) {
      const parsed = describePrivateKey(key).parsed;
      return parsed ? `${parsed.keyType} ${parsed.publicBlob.toString('base64')}` : '';
    },
    generateSshKey: options => invoke('ssh_generate_key', {
      options: {
        type: String(options?.type ?? 'ed'),
        bits: Number(options?.bits) || null,
        curve: String(options?.curve ?? ''),
        password: String(options?.password ?? ''),
        comment: String(options?.comment ?? ''),
      },
    }),
  };
}
