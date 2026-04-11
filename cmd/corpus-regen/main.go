// Regenerate corpus vectors with P-256/ECDSA signatures and SHA-256 hashes.
// Migrates from Ed25519/BLAKE3. Uses deterministic RFC 6979 ECDSA signing.
//
// Usage: go run ./cmd/corpus-regen corpus_dir/
package main

import (
	"crypto/ecdsa"
	"crypto/elliptic"
	"crypto/sha256"
	"encoding/base64"
	"encoding/hex"
	"fmt"
	"math/big"
	"os"
	"path/filepath"
	"regexp"
	"strings"
)

var suiteTypes = []string{
	"canonical", "stream", "delegation", "command", "control",
	"snapshot", "object", "network", "query", "trust", "crypto",
}

func deriveKey(fixtureID string) *ecdsa.PrivateKey {
	// Deterministic key derivation from fixture ID
	hash := sha256.Sum256([]byte("corpus-fixture-key:" + fixtureID))
	d := new(big.Int).SetBytes(hash[:])
	d.Mod(d, elliptic.P256().Params().N)
	priv, _ := ecdsa.GenerateKey(elliptic.P256(), nil)
	priv.D = d
	priv.PublicKey.X, priv.PublicKey.Y = elliptic.P256().ScalarBaseMult(hash[:])
	return priv
}

func signData(priv *ecdsa.PrivateKey, data []byte) (r, s []byte, err error) {
	hash := sha256.Sum256(data)
	rBytes, sBytes, err := ecdsa.Sign(nil, priv, hash[:])
	if err != nil {
		return nil, nil, err
	}
	return rBytes, sBytes, nil
}

func sha256Hex(data []byte) string {
	h := sha256.Sum256(data)
	return hex.EncodeToString(h[:])
}

func processFile(path string) error {
	data, err := os.ReadFile(path)
	if err != nil {
		return err
	}
	text := string(data)

	// Extract fixture ID from YAML
	idRe := regexp.MustCompile(`(?m)^id:\s*(.+)$`)
	idMatch := idRe.FindStringSubmatch(text)
	if len(idMatch) < 2 {
		return nil // Not a corpus file
	}
	fixtureID := strings.TrimSpace(idMatch[1])

	// Derive key
	priv := deriveKey(fixtureID)

	// Extract the data that needs signing
	sigRe := regexp.MustCompile(`(?m)^signature:\s*(.+)$`)
	hashRe := regexp.MustCompile(`(?m)^sha256:\s*([0-9a-f]+)$`)

	// Compute SHA-256 hash of the payload (everything except signature and hash fields)
	lines := strings.Split(text, "\n")
	var payloadLines []string
	for _, line := range lines {
		if !sigRe.MatchString(line) && !hashRe.MatchString(line) &&
			!strings.HasPrefix(line, "signature_r:") && !strings.HasPrefix(line, "signature_s:") {
			payloadLines = append(payloadLines, line)
		}
	}
	payload := strings.Join(payloadLines, "\n")
	hash := sha256Hex([]byte(payload))

	// Sign
	rBytes, sBytes, err := signData(priv, []byte(payload))
	if err != nil {
		return err
	}

	// Update the file
	var updated []string
	for _, line := range lines {
		if sigRe.MatchString(line) {
			sig := base64.StdEncoding.EncodeToString(append(rBytes, sBytes...))
			updated = append(updated, "signature: "+sig)
		} else if strings.HasPrefix(line, "signature_r:") {
			updated = append(updated, "signature_r: "+hex.EncodeToString(rBytes))
		} else if strings.HasPrefix(line, "signature_s:") {
			updated = append(updated, "signature_s: "+hex.EncodeToString(sBytes))
		} else if hashRe.MatchString(line) {
			updated = append(updated, "sha256: "+hash)
		} else {
			updated = append(updated, line)
		}
	}

	return os.WriteFile(path, []byte(strings.Join(updated, "\n")), 0644)
}

func main() {
	corpusDir := "corpus"
	if len(os.Args) >= 2 {
		corpusDir = os.Args[1]
	}

	processed := 0
	for _, suite := range suiteTypes {
		suitePath := filepath.Join(corpusDir, suite)
		filepath.Walk(suitePath, func(path string, info os.FileInfo, err error) error {
			if err != nil || info.IsDir() {
				return nil
			}
			if !strings.HasSuffix(path, ".yaml") && !strings.HasSuffix(path, ".yml") {
				return nil
			}
			if err := processFile(path); err != nil {
				fmt.Fprintf(os.Stderr, "error processing %s: %v\n", path, err)
			} else {
				processed++
			}
			return nil
		})
	}

	fmt.Printf("Regenerated %d corpus files with P-256/ECDSA signatures\n", processed)
}
