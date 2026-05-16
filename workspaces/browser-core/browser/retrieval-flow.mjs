import {
  createBrowserPackageRetrievalAdmissionHash,
  createBrowserPackageRetrievalEvidence,
  createBrowserPackageRetrievalRecord,
} from "./package-retrieval.mjs";
import { storeRetrievedNetworkAppPackage } from "./package-source.mjs";

export async function persistAdmittedNetworkAppRetrieval(host, store, selection = {}, retrieval = {}) {
  if (!host?.exports) {
    throw new TypeError("admitted network app retrieval requires a browser core host");
  }
  if (!store) {
    throw new TypeError("admitted network app retrieval requires a browser byte store");
  }
  const packageKey = packageSelectionKey(selection);
  const packageBytes = retrievalPackageBytes(retrieval);
  const retrievalEvidenceBytes =
    retrieval.retrievalEvidenceBytes ??
    retrieval.retrievalRecordBytes ??
    createBrowserPackageRetrievalEvidence(host.exports, packageBytes, {
      packageKey,
      retrievalCost: retrieval.retrievalCost,
      retrievedAt: retrieval.retrievedAt,
      policyScheduleBytes: retrieval.policyScheduleBytes,
    });
  const sourceAdmissionHash =
    retrieval.browserAdmissionHash ??
    retrieval.sourceAdmissionHash ??
    createBrowserPackageRetrievalAdmissionHash(host.exports, packageBytes, {
      packageKey,
      retrievalCost: retrieval.retrievalCost,
      retrievedAt: retrieval.retrievedAt,
      policyScheduleBytes: retrieval.policyScheduleBytes,
    });
  const recordBytes =
    retrieval.recordBytes ??
    createBrowserPackageRetrievalRecord(host.exports, packageBytes, {
      packageKey,
      retrievalCost: retrieval.retrievalCost,
      retrievedAt: retrieval.retrievedAt,
      sourceAdmissionHash,
      retrievalEvidenceBytes,
      proofBytes: retrieval.proofBytes,
    });

  await storeRetrievedNetworkAppPackage(
    store,
    { ...selection, packageKey },
    {
      ...retrieval,
      browserAdmissionHash: sourceAdmissionHash,
      browserWorkAdmissionBytes:
        retrieval.browserWorkAdmissionBytes ?? retrieval.workAdmissionBytes,
      retrievalEvidenceBytes,
      sourceAdmissionHash,
      recordBytes,
    },
  );

  return {
    packageBytes,
    policyScheduleBytes: retrieval.policyScheduleBytes,
    retrievalEvidenceBytes,
    browserAdmissionHash: sourceAdmissionHash,
    sourceAdmissionHash,
    browserWorkAdmissionBytes: retrieval.browserWorkAdmissionBytes ?? retrieval.workAdmissionBytes,
    workAdmissionBytes: retrieval.browserWorkAdmissionBytes ?? retrieval.workAdmissionBytes,
    recordBytes,
  };
}

function retrievalPackageBytes(retrieval) {
  return {
    manifestBytes: retrieval.manifestBytes,
    graphBytes: retrieval.graphBytes,
    developerSignatureBytes: retrieval.developerSignatureBytes,
  };
}

function packageSelectionKey(selection) {
  const packageKey = selection?.packageKey ?? selection?.key;
  if (!packageKey) {
    throw new Error("retrieved network app package key is required");
  }
  return packageKey;
}
