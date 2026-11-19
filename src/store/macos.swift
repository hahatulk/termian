import CryptoKit
import Foundation
import LocalAuthentication
import Security
import Dispatch

private let purpose = Data("Termian local vault key v2".utf8)
private let publicSize = 65
private let boxSize = 12 + 32 + 16

private func wrappingKey(_ secret: SharedSecret, publicKey: Data) -> SymmetricKey {
    secret.hkdfDerivedSymmetricKey(
        using: SHA256.self, salt: publicKey, sharedInfo: purpose, outputByteCount: 32)
}

@_cdecl("termian_vault_wrap")
func wrap(
    _ key: UnsafeRawPointer, _ output: UnsafeMutableRawPointer, _ capacity: Int,
    _ length: UnsafeMutablePointer<Int>
) -> Int32 {
    guard SecureEnclave.isAvailable else { return 1 }
    guard let access = SecAccessControlCreateWithFlags(
        nil, kSecAttrAccessibleWhenUnlockedThisDeviceOnly, [.privateKeyUsage, .userPresence], nil
    ) else { return 2 }

    do {
        let device = try SecureEnclave.P256.KeyAgreement.PrivateKey(accessControl: access)
        let ephemeral = P256.KeyAgreement.PrivateKey()
        let publicKey = ephemeral.publicKey.x963Representation
        let secret = try ephemeral.sharedSecretFromKeyAgreement(with: device.publicKey)
        let input = UnsafeRawBufferPointer(start: key, count: 32)
        guard let box = try AES.GCM.seal(input, using: wrappingKey(secret, publicKey: publicKey)).combined else {
            return 2
        }
        let wrapped = device.dataRepresentation + publicKey + box
        guard wrapped.count <= capacity else { return 2 }
        wrapped.copyBytes(to: output.assumingMemoryBound(to: UInt8.self), count: wrapped.count)
        length.pointee = wrapped.count
        return 0
    } catch {
        return 2
    }
}

@_cdecl("termian_vault_unwrap")
func unwrap(
    _ input: UnsafeRawPointer, _ length: Int, _ key: UnsafeMutableRawPointer,
    _ allowInteraction: Int32
) -> Int32 {
    guard SecureEnclave.isAvailable else { return 1 }
    guard length > publicSize + boxSize else { return 3 }
    let wrapped = Data(bytes: input, count: length)
    let deviceEnd = length - publicSize - boxSize
    let publicEnd = length - boxSize
    let context = LAContext()
    context.localizedReason = "Open the Termian vault"
    context.touchIDAuthenticationAllowableReuseDuration = 0
    context.interactionNotAllowed = allowInteraction == 0
    defer { context.invalidate() }

    do {
        let device = try SecureEnclave.P256.KeyAgreement.PrivateKey(
            dataRepresentation: wrapped.prefix(deviceEnd), authenticationContext: context)
        let publicKey = wrapped[deviceEnd..<publicEnd]
        let ephemeral = try P256.KeyAgreement.PublicKey(x963Representation: publicKey)
        let box = try AES.GCM.SealedBox(combined: wrapped.suffix(boxSize))
        let secret = try device.sharedSecretFromKeyAgreement(with: ephemeral)
        var plain = try AES.GCM.open(box, using: wrappingKey(secret, publicKey: publicKey))
        defer { plain.resetBytes(in: plain.startIndex..<plain.endIndex) }
        guard plain.count == 32 else { return 3 }
        plain.copyBytes(to: key.assumingMemoryBound(to: UInt8.self), count: 32)
        return 0
    } catch let error as LAError {
        switch error.code {
        case .userCancel, .systemCancel, .appCancel:
            return 4
        case .notInteractive:
            return 5
        default:
            return 6
        }
    } catch {
        return 6
    }
}

@_cdecl("termian_privacy_available")
func privacyAvailable(_ biometric: Int32) -> Int32 {
    let policy: LAPolicy = biometric != 0 ? .deviceOwnerAuthenticationWithBiometrics : .deviceOwnerAuthentication
    return LAContext().canEvaluatePolicy(policy, error: nil) ? 1 : 0
}

@_cdecl("termian_privacy_verify")
func privacyVerify(_ biometric: Int32) -> Int32 {
    let context = LAContext()
    context.touchIDAuthenticationAllowableReuseDuration = 0
    let policy: LAPolicy = biometric != 0 ? .deviceOwnerAuthenticationWithBiometrics : .deviceOwnerAuthentication
    let done = DispatchSemaphore(value: 0)
    var passed = false
    context.evaluatePolicy(policy, localizedReason: "Unlock Termian") { success, _ in
        passed = success
        done.signal()
    }
    done.wait()
    context.invalidate()
    return passed ? 1 : 0
}
