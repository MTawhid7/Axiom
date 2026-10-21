# S15 Results: a Stable Local Signing Identity

| | |
|---|---|
| **Status** | **Complete** (2026-10-21). **S15-Q1 passes:** under a self-signed identity, S4's scenarios behave exactly as under ad-hoc signing: the probes, the JIT matrix with `allow-jit` and `single-jit`, CoreText in the text profile, start-up and IPC. **S15-Q2 passes for the Keychain:** a rebuilt binary signed with the same identity reads the first build's Keychain item without any dialog, where an ad-hoc rebuild is refused; the item trusts the identity's designated requirement, which the rebuild satisfies. **Privacy decisions** (TCC) weren't tested: they need the shell as an app bundle and the owner's clicks, at the shell's checkpoint. **Recommended:** sign Axiom with a stable local identity from Milestone 1 (ADR-0004 D2, as amended) |
| **Question** | Under a self-signed code-signing identity with an explicit designated requirement, do S4's probes, the JIT entitlements and the custom profiles behave as under ad-hoc signing (S15-Q1)? Do decisions macOS records about the code, Keychain approvals and privacy permissions, persist across rebuilds (S15-Q2)? |
| **Feeds** | [ADR-0004](../adr/0004-process-model-and-sandbox.md) D2; the [process model](../security/process-model.md) §10; [Milestone 1](../milestone-1.md)'s MA10 and MA12; the architecture review's [RV1-17](../review/round-1/02-findings.md#rv1-17-the-signing-identity) |
| **Plan** | The review's [actions](../review/round-1/04-round-2-actions.md#4-proposed-spikes) §4, authorized by the owner on 2026-10-21: "Spike S15: Stable self-signed codesigning persistence across rebuilds." RV1-17's disposition: "Validate local self-signed codesigning persistence via Spike S15." |
| **Code** | `spikes/s15/probe` (`unsafe` only in `src/platform/`, Security's legacy keychain functions); `tools/s15-measure.sh`; `tools/s4-sign.sh`'s new `--identity` and `--keychain` |
| **Last updated** | 2026-10-21 |

**Evidence labels** follow the [research conventions](README.md): **measured** facts come from the runs in §2; **observed** facts were read on this Mac, naming the file or command; **sourced** facts link their source; **inference** is marked.

## 1. Why

Apple's TN3127 (revised 2024-04-02): "Ad hoc signed code … has a DR but it's tied to that specific version of the code. In both cases macOS can't reliably track the identity of the code", so "the system fails to remember your choices during development" for privacy-protected resources, and the keychain "presents unexpected authorization alerts" ([RV1-17](../review/round-1/02-findings.md#rv1-17-the-signing-identity), source [A1] there). Axiom rebuilds whenever an advisory touches a crate that parses untrusted input (TS15), so ad-hoc signing would ask the owner again after each rebuild, and train them to click "Always Allow".

## 2. The runs

`tools/s15-measure.sh setup s4 keychain requirement`, 2026-10-21 14:59–15:00, with the script's three fixes rerun at 15:00 (`target/s15-results/20261021-150041/`), then `teardown` (`20261021-150121/`). macOS 27.2, the Command Line Tools' `codesign`, LibreSSL 3.3.6 (observed).

**The identity** (observed):
- **Made** by `setup`: a key pair and a self-signed certificate from LibreSSL, `CN=Axiom Local Code Signing (S15)`, with `extendedKeyUsage = critical, codeSigning`, in a throwaway keychain file, `target/s15/keychain/s15.keychain-db`, under a random password. Its key's partition list, `apple-tool:,apple:`, lets `codesign` use it without a dialog (`security set-key-partition-list`).
- **Untrusted, and enough:** `security find-identity` lists it as `CSSMERR_TP_NOT_TRUSTED`, with 0 valid identities, yet `codesign --sign 052C4E8F5380D876F006EFF33D41AB45E1F55AF9 --keychain …` signs with it, and `codesign --verify --strict` reports "valid on disk" and "satisfies its Designated Requirement". No trust setting was changed, so nothing outside the keychain file was touched.
- **Its designated requirement:** `identifier "org.axiom.s4.helper" and certificate root = H"052c4e8f5380d876f006eff33d41ab45e1f55af9"`, the identifier and the certificate's hash, with no code hash; `TeamIdentifier=not set`. Ad hoc, the same binary's is `cdhash H"…"`.

### 2.1 S15-Q1: S4's scenarios under each signing

`tools/s4-sign.sh` signed S4's six binaries twice, ad hoc and with the identity (`--identity`, `--keychain`); the identity-signed helpers carry their entitlements, `allow-jit` on `s4-helper-jit`, and `allow-jit` with `single-jit` on `s4-helper-jit1` (observed in `sign-identity.txt`). Then `s4-broker probe`, `jit`, `text --runs 2`, `startup --runs 5` and `ipc --batches 2` ran against each set (measured):

| Scenario | Ad hoc | Identity | Output, with every number masked |
|---|---|---|---|
| `probe`: S4's 18 probes, the control and the renderer profile by each method | Exit 0 | Exit 0 | The same 116 lines, but for the first, which names the helpers' directory |
| `jit`: the JIT matrix, four executables × four profiles | Exit 0 | Exit 0 | The same, but for the first line: `allow-jit` maps JIT memory unless the profile denies it explicitly (J3); `single-jit` refuses a second region (J5) |
| `text`: CoreText inside the text profile | Exit 0 | Exit 0 | The same, but for the first line |
| `startup`, `ipc` | Exit 0 | Exit 0 | The same, but for the first line |

Each `s4-broker` command exits 0 only if every check it makes holds.

### 2.2 S15-Q2: a rebuild and the Keychain

For each signing, `s15-probe` was built with the tag `v1`, signed, and run to add a generic password to the S15 keychain as the app that creates it; then built with the tag `v2`, which changes its code and so its code hash, signed the same way, put at the same path, and run to read the password. Both builds disallow the keychain's interface first (`SecKeychainSetUserInteractionAllowed(false)`), so a refusal is a status, never a dialog (measured):

| Signing | v1 adds | v1 reads | **v2, rebuilt, reads** | The item's access list trusts |
|---|---|---|---|---|
| Ad hoc | 0 | 0 | **-25293** (`errSecAuthFailed`): refused | `cdhash H"ecbcd3b4…"`: v1 alone |
| Identity | 0 | 0 | **0**, v1's secret | `identifier "org.axiom.s15.probe" and certificate root = H"052c4e8f…"` |

**The requirement check agrees** (measured): `codesign --verify -R "=<v1's designated requirement>"` passes on v2 under the identity, and fails ad hoc.

So **the Keychain tracks the code by its designated requirement,** as TN3127 says, and a stable identity keeps a rebuilt browser's access to its own items (measured, in a keychain file; that the login keychain behaves the same, being the same kind of file-based keychain, is inference).

### 2.3 Privacy decisions: not tested

TCC, which records permissions such as the microphone's, identifies code by its designated requirement too (TN3127, as RV1-17 quotes it), so the identity should keep them (inference). **Testing it needs** the owner to answer a permission prompt, then a rebuild. It also needs the shell as an app bundle: macOS attributes a command-line tool's privacy requests to its responsible process, such as Terminal (background), so a bare executable can't hold its own permission. **Left to Milestone 1's shell checkpoint,** by the owner's hand.

## 3. What it means

- **ADR-0004 D2's condition is met** for S4's probes and the JIT entitlements (S15-Q1) and for the Keychain (S15-Q2). The recommendation: sign the browser and its helpers with a stable local identity from Milestone 1, with ad-hoc signing kept as the fallback.
- **For Milestone 1's plan** (MA10, MA12):
  - the owner creates the identity in their login keychain at M1's start, as `tools/s15-measure.sh setup` does in a file: a self-signed certificate with the code-signing usage, whose key's partition list admits `codesign`; no trust setting is needed;
  - the signing script names it by its SHA-1; the private key never enters the repository;
  - the designated requirement stays the default, identifier and certificate root;
  - TCC is checked by hand at the shell's checkpoint (§2.3).
- **For the Keychain at M6:** the legacy `SecKeychain` functions this spike used are deprecated since macOS 10.10, though SDK 27 still declares them and `Security.tbd` exports them (observed); M6's design should use `SecItem` on the same file-based keychain (inference).

## 4. Side effects

**None outside `target/`** (observed):
- the keychain file was created under `target/s15/keychain/` and deleted by `teardown`, which also deleted the private key, the PKCS #12 file and the passwords;
- the user's keychain search list, the login keychain alone, was the same before, after `setup` and after `teardown`;
- the login keychain and the system's trust settings were never opened or changed;
- no crash report was written.
