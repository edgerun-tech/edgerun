/**
 * Qwen OAuth Integration Test
 * Tests the Qwen Code OAuth flow and API connection
 */

const QWEN_TOKEN = {
  access_token: "miWnMkrdycbATyXT0m9IUbMeuc1pFZVnofQh9WzXQ0PJnACY3a4dPaaSuQPYJSI0-bi-aKSITsZqNnbuOOo7YA",
  token_type: "Bearer",
  refresh_token: "Jjod6cfmZLRP_bGxXHbHbu8hdjwAUcOIey4GT4fdRp4h30R79PbSL4XOezo4Z1ru7ou1hLpfb9ycQsoiwJaoDw",
  resource_url: "portal.qwen.ai",
  expiry_date: 1771325484237
};

const QWEN_API_BASE = "https://dashscope.aliyuncs.com/compatible-mode/v1";

async function testQwenAPI() {
  console.log("🧪 Testing Qwen Code OAuth Integration...\n");

  // Step 1: Store token in localStorage format
  console.log("1. Storing OAuth token...");
  const tokenStr = JSON.stringify(QWEN_TOKEN);
  console.log("   ✓ Token stored:", tokenStr.substring(0, 50) + "...");

  // Step 2: Test models endpoint
  console.log("\n2. Testing /models endpoint...");
  try {
    const modelsResponse = await fetch(`${QWEN_API_BASE}/models`, {
      method: 'GET',
      headers: {
        'Authorization': `Bearer ${QWEN_TOKEN.access_token}`,
        'Content-Type': 'application/json'
      }
    });

    if (modelsResponse.ok) {
      const models = await modelsResponse.json();
      console.log("   ✓ Models endpoint successful!");
      console.log("   Available models:", JSON.stringify(models, null, 2).substring(0, 200));
    } else {
      const error = await modelsResponse.text();
      console.log("   ✗ Models endpoint failed:", modelsResponse.status, error);
    }
  } catch (error) {
    console.log("   ✗ Models endpoint error:", error.message);
  }

  // Step 3: Test chat completions endpoint
  console.log("\n3. Testing /chat/completions endpoint...");
  try {
    const chatResponse = await fetch(`${QWEN_API_BASE}/chat/completions`, {
      method: 'POST',
      headers: {
        'Authorization': `Bearer ${QWEN_TOKEN.access_token}`,
        'Content-Type': 'application/json'
      },
      body: JSON.stringify({
        model: "qwen-plus",
        messages: [
          { role: "system", content: "You are a helpful assistant." },
          { role: "user", content: "Say hello in one short sentence." }
        ],
        max_tokens: 50,
        stream: false
      })
    });

    if (chatResponse.ok) {
      const result = await chatResponse.json();
      console.log("   ✓ Chat completions endpoint successful!");
      console.log("   Response:", JSON.stringify(result, null, 2).substring(0, 300));
    } else {
      const error = await chatResponse.text();
      console.log("   ✗ Chat completions endpoint failed:", chatResponse.status, error);
    }
  } catch (error) {
    console.log("   ✗ Chat completions endpoint error:", error.message);
  }

  // Step 4: Verify token structure
  console.log("\n4. Verifying token structure...");
  const requiredFields = ['access_token', 'token_type', 'refresh_token', 'resource_url', 'expiry_date'];
  const missingFields = requiredFields.filter(field => !(field in QWEN_TOKEN));
  
  if (missingFields.length === 0) {
    console.log("   ✓ All required fields present");
  } else {
    console.log("   ✗ Missing fields:", missingFields.join(", "));
  }

  // Check expiry
  const now = Date.now();
  if (QWEN_TOKEN.expiry_date > now) {
    const daysLeft = Math.round((QWEN_TOKEN.expiry_date - now) / (1000 * 60 * 60 * 24));
    console.log(`   ✓ Token valid for ${daysLeft} more days`);
  } else {
    console.log("   ✗ Token has expired!");
  }

  console.log("\n✅ Qwen OAuth Integration Test Complete\n");
}

// Run the test
testQwenAPI().catch(console.error);
