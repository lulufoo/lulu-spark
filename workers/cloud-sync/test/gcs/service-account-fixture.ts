import { exportPKCS8, generateKeyPair } from "jose";

export const CLIENT_EMAIL = "test-sync@test-project.iam.gserviceaccount.com";
export const TOKEN_URI = "https://oauth2.test/token";

/** A fresh RSA key pair shaped like a downloaded service account key file. */
export async function makeServiceAccount() {
  const { publicKey, privateKey } = await generateKeyPair("RS256", { extractable: true });
  const privateKeyPem = await exportPKCS8(privateKey);
  const keyJson = JSON.stringify({
    type: "service_account",
    project_id: "test-project",
    private_key_id: "0123456789abcdef",
    private_key: privateKeyPem,
    client_email: CLIENT_EMAIL,
    token_uri: TOKEN_URI,
  });
  return { publicKey, privateKeyPem, keyJson };
}
