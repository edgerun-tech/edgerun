//! Policy-wrapping decorator for capability providers.

use crate::collections::HashMap;
use crate::prelude::v1::*;

use edgerun_capabilities::{
    CapabilityDescriptor, CapabilityError, PolicyContext, PolicyDecision, PolicyEngine,
    RevocationReason, SimplePolicyEngine,
};
use edgerun_protocols::core_protocol::protocol::capability::{
    CapabilityGrant, CapabilityRequest, CapabilityRevocation,
};
use edgerun_protocols::core_protocol::protocol::capability_runtime::{
    CapabilitySessionAccept, CapabilitySessionClose, CapabilitySessionOpen,
};

use crate::protocol::{
    RemoteCapabilityProvider, RemoteInvocationResult, accept_session_open_unchecked,
    default_remote_requester_opt, session_accept_from_grant, session_open_as_request,
    session_reject,
};

/// Session-to-grant association record.
#[derive(Clone, Debug)]
pub struct SessionGrantBinding {
    pub session_id: Vec<u8>,
    pub grant_id: Vec<u8>,
    pub granted_operations: Vec<i32>,
    pub granted_access_class: i32,
}

/// Wraps any `RemoteCapabilityProvider` with policy enforcement.
#[derive(Debug)]
pub struct PolicyWrappedProvider<P> {
    inner: P,
    policy: SimplePolicyEngine,
    context: PolicyContext,
    /// Sessions keyed by grant_id (used by invocations).
    sessions: HashMap<Vec<u8>, SessionGrantBinding>,
    /// Reverse index: session_id → grant_id (used by close_session).
    session_to_grant: HashMap<Vec<u8>, Vec<u8>>,
}

impl<P> PolicyWrappedProvider<P> {
    pub fn new(inner: P) -> Self {
        Self {
            inner,
            policy: SimplePolicyEngine::default(),
            context: PolicyContext {
                is_local: false,
                ..PolicyContext::default()
            },
            sessions: HashMap::new(),
            session_to_grant: HashMap::new(),
        }
    }

    pub fn with_policy(inner: P, policy: SimplePolicyEngine) -> Self {
        Self {
            inner,
            policy,
            context: PolicyContext {
                is_local: false,
                ..PolicyContext::default()
            },
            sessions: HashMap::new(),
            session_to_grant: HashMap::new(),
        }
    }

    pub fn with_context(mut self, context: PolicyContext) -> Self {
        self.context = context;
        self
    }

    pub fn inner(&self) -> &P {
        &self.inner
    }

    pub fn inner_mut(&mut self) -> &mut P {
        &mut self.inner
    }

    pub fn policy(&self) -> &SimplePolicyEngine {
        &self.policy
    }

    pub fn policy_mut(&mut self) -> &mut SimplePolicyEngine {
        &mut self.policy
    }

    pub fn context(&self) -> &PolicyContext {
        &self.context
    }

    pub fn sessions(&self) -> &HashMap<Vec<u8>, SessionGrantBinding> {
        &self.sessions
    }

    fn request_for_open(
        &self,
        descriptor: &CapabilityDescriptor,
        open: &CapabilitySessionOpen,
    ) -> CapabilityRequest {
        session_open_as_request(
            open,
            descriptor,
            self.context
                .requester
                .clone()
                .or_else(default_remote_requester_opt),
            self.context.requester_node.clone(),
        )
    }

    fn policy_grant_for_request(
        &mut self,
        descriptor: &CapabilityDescriptor,
        request: &CapabilityRequest,
    ) -> Result<CapabilityGrant, CapabilityError> {
        let decision = self
            .policy
            .evaluate_request(descriptor, request, &self.context)?;
        match decision {
            PolicyDecision::Allow { grant } => Ok(*grant),
            PolicyDecision::Deny { reason } | PolicyDecision::RequireInteraction { reason } => {
                Err(CapabilityError::PermissionDenied(reason))
            }
        }
    }
}

impl<P> RemoteCapabilityProvider for PolicyWrappedProvider<P>
where
    P: RemoteCapabilityProvider,
{
    fn descriptor(&self) -> CapabilityDescriptor {
        self.inner.descriptor()
    }

    fn open_session(
        &mut self,
        open: &CapabilitySessionOpen,
    ) -> Result<CapabilitySessionAccept, CapabilityError> {
        let descriptor = self.inner.descriptor();
        let request = self.request_for_open(&descriptor, open);
        let grant = match self.handle_request(&request) {
            Ok(Some(grant)) => grant,
            Ok(None) => {
                return Ok(session_reject(
                    open,
                    "policy provider did not return a grant for the session request",
                ));
            }
            Err(CapabilityError::PermissionDenied(reason)) => {
                return Ok(session_reject(open, reason));
            }
            Err(err) => return Err(err),
        };

        let mut accept = self.inner.open_session(open)?;
        if !accept.accepted {
            let _ = self
                .policy
                .revoke(&grant.grant_id, RevocationReason::Superseded);
            return Ok(accept);
        }

        let standardized_accept = session_accept_from_grant(open, &grant);
        accept.granted_operations = standardized_accept.granted_operations;
        accept.granted_access_class = standardized_accept.granted_access_class;
        accept.grant_id = standardized_accept.grant_id;
        let binding = SessionGrantBinding {
            session_id: open.session_id.clone(),
            grant_id: grant.grant_id.clone(),
            granted_operations: grant.granted_operations.clone(),
            granted_access_class: grant.access_class,
        };
        self.sessions.insert(grant.grant_id.clone(), binding);
        self.session_to_grant
            .insert(open.session_id.clone(), grant.grant_id);
        Ok(accept)
    }

    fn invoke(
        &mut self,
        session_id: &[u8],
        invocation: &edgerun_protocols::core_protocol::protocol::capability::CapabilityInvocation,
        inline_parameters: Option<&[u8]>,
    ) -> Result<RemoteInvocationResult, CapabilityError> {
        let binding = self
            .sessions
            .get(session_id)
            .ok_or(CapabilityError::PermissionDenied(
                "capability session has no active policy grant",
            ))?;
        let mut policy_invocation = invocation.clone();
        policy_invocation.grant_id = binding.grant_id.clone();
        self.policy
            .authorize_invocation(&policy_invocation, &self.context)?;
        self.inner.invoke(session_id, invocation, inline_parameters)
    }

    fn next_event(
        &mut self,
        session_id: &[u8],
    ) -> Result<
        Option<
            edgerun_protocols::core_protocol::protocol::capability_runtime::CapabilitySessionEvent,
        >,
        CapabilityError,
    > {
        self.inner.next_event(session_id)
    }

    fn close_session(&mut self, close: &CapabilitySessionClose) -> Result<(), CapabilityError> {
        if let Some(grant_id) = self.session_to_grant.remove(&close.session_id) {
            if let Some(binding) = self.sessions.remove(&grant_id) {
                let _ = self
                    .policy
                    .revoke(&binding.grant_id, RevocationReason::Superseded);
            }
        }
        self.inner.close_session(close)
    }

    fn handle_request(
        &mut self,
        request: &CapabilityRequest,
    ) -> Result<Option<CapabilityGrant>, CapabilityError> {
        let descriptor = self.inner.descriptor();
        self.policy_grant_for_request(&descriptor, request)
            .map(Some)
    }

    fn handle_revocation(
        &mut self,
        revocation: &CapabilityRevocation,
    ) -> Result<(), CapabilityError> {
        self.policy.apply_revocation(revocation.clone())
    }
}

/// Blanket impl for wrapping providers in policy.
pub trait IntoPolicyWrappedProvider: RemoteCapabilityProvider + Sized {
    fn into_policy_wrapped(self) -> PolicyWrappedProvider<Self> {
        PolicyWrappedProvider::new(self)
    }

    fn into_policy_wrapped_with(
        self,
        policy: SimplePolicyEngine,
        context: PolicyContext,
    ) -> PolicyWrappedProvider<Self> {
        PolicyWrappedProvider::with_policy(self, policy).with_context(context)
    }
}

impl<T> IntoPolicyWrappedProvider for T where T: RemoteCapabilityProvider {}
