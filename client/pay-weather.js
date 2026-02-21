import "dotenv/config";
import { wrapFetchWithPayment, x402Client, x402HTTPClient } from "@x402/fetch";
import { registerExactEvmScheme } from "@x402/evm/exact/client";
import { privateKeyToAccount } from "viem/accounts";

const url = process.env.RESOURCE_SERVER_URL || "http://localhost:8080";
const endpoint = process.env.ENDPOINT_PATH || "/api/weather";
const fullUrl = `${url}${endpoint}`;

const signer = privateKeyToAccount(process.env.EVM_PRIVATE_KEY);
const client = new x402Client();
registerExactEvmScheme(client, { signer });

const fetchWithPayment = wrapFetchWithPayment(fetch, client);

console.log(`Making request to: ${fullUrl}\n`);
const res = await fetchWithPayment(fullUrl, { method: "GET" });
const data = await res.json();
console.log("Response:", data);

if (res.ok) {
  const httpClient = new x402HTTPClient(client);
  try {
    const paymentResponse = httpClient.getPaymentSettleResponse((n) =>
      res.headers.get(n)
    );
    console.log("Payment settled:", paymentResponse);
  } catch (_) {
    console.log("(Payment settled, receipt header not present)");
  }
} else {
  console.log(`\nNo payment settled (response status: ${res.status})`);
}
