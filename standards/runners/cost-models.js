const crypto = require("crypto");

const COST_MODELS = {
  "sha1-fips180": {
    privacy: "length-only",
    blockSize: 64,
    lengthFieldBytes: 8,
    base: 32,
    perBlock: 80,
  },
  "sha256-fips180": {
    privacy: "length-only",
    blockSize: 64,
    lengthFieldBytes: 8,
    base: 32,
    perBlock: 96,
  },
  "sha384-fips180": {
    privacy: "length-only",
    blockSize: 128,
    lengthFieldBytes: 16,
    base: 48,
    perBlock: 160,
  },
  "sha512-fips180": {
    privacy: "length-only",
    blockSize: 128,
    lengthFieldBytes: 16,
    base: 48,
    perBlock: 160,
  },
};

function sha256Hex(bytes) {
  return crypto.createHash("sha256").update(bytes).digest("hex");
}

function hashBlocks(inputLen, model) {
  return Math.ceil((inputLen + 1 + model.lengthFieldBytes) / model.blockSize);
}

function hashCost(unitId, inputLen) {
  const model = COST_MODELS[unitId];
  if (!model) {
    throw new Error(`missing cost model for ${unitId}`);
  }
  const blocks = hashBlocks(inputLen, model);
  return {
    unit: unitId,
    privacy: model.privacy,
    input_len: inputLen,
    blocks,
    cost: model.base + blocks * model.perBlock,
  };
}

module.exports = {
  COST_MODELS,
  hashBlocks,
  hashCost,
  sha256Hex,
};
