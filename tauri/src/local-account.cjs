'use strict';

// A Redux-only identity for the permanently offline desktop build.  This module
// must never write Settings, CredentialsStorage, the keychain, or API tokens.
const LOCAL_ACCOUNT_ACTION = 'SELF_CLOUD_LOCAL_ACCOUNT_RESTORE';
const BLOCKED_ACTION_TYPES = new Set([
  // These requests have sagas which delete credentials, Settings, Store data,
  // local records, or call account endpoints.
  'LOGOUT_REQUEST',
  'DELETE_ACCOUNT_REQUEST',
  'CLEAN_DATABASE_REQUEST',
  'REMOVE_LOCAL_DATA_REQUEST',
  'LOGIN_REQUEST',
  'SIGN_UP_FREE_REQUEST',
  'SIGNUP_TRIAL_REQUEST',
  'CHANGE_USERNAME_REQUEST',
  'CHANGE_PASSWORD_REQUEST',
  'RECOVER_ACCOUNT_REQUEST',
  'RESET_PASSWORD_REQUEST',
  // Never permit a late/externally supplied completion to reset the user slice.
  'LOGOUT_SUCCESS',
  // This is a personal local client: neither invitations nor team provisioning
  // should start a saga, even if a stale/deep-linked UI dispatches the action.
  ...[
    'CREATE_TEAM_TRIAL', 'MODIFY_TEAM', 'INVITE_TEAM_MEMBER', 'RESEND_TEAM_INVITE',
    'GET_TEAM_SEATS_LIST', 'GET_TEAM_MEMBERS_LIST', 'GET_TEAM_INVITES_LIST',
    'GET_TEAM_MEMBER_STALE_ENCRYPTION_LIST', 'GENERATE_INVITATION_LINK',
    'SIGN_UP_IN_TEAM_TRIAL_ONBOARDING', 'CREATE_TEAM_IN_TEAM_TRIAL_ONBOARDING',
    'CREATE_TEAM_IN_CHOOSE_PLAN_DIALOG', 'GRANT_ACCESS_TO_VAULTS',
    'REJECT_ACCESS_TO_VAULTS', 'LEAVE_TEAM', 'START_TEAM_TRIAL_GRANT_ACCESS_EXPERIMENT',
    'EXTEND_TEAM_TRIAL',
  ].flatMap(type => [`${type}_REQUEST`, `${type}_SUCCESS`]),
]);

function deepFreeze(value) {
  if (!value || typeof value !== 'object' || Object.isFrozen(value)) return value;
  Object.freeze(value);
  for (const child of Object.values(value)) deepFreeze(child);
  return value;
}

// Mirrors the complete 9.35.4 user reducer initial shape.  Null/false values are
// intentional: a local identity is not a Termius account and grants no paid,
// team, trial, subscription, or cloud entitlement.
const LOCAL_USER = deepFreeze({
  username: 'Local user',
  team: null,
  isTeamDataStale: false,
  personal_subscription: null,
  team_subscription: null,
  student: null,
  trial: null,
  account: {
    id: null,
    user_id: null,
    is_email_confirmed: true,
    need_to_update_subscription: false,
    pro_mode: false,
    two_factor_auth: false,
    team: false,
    updated_at: '',
    feature_toggles: null,
    registered_at: '',
    now: '',
    plan_type: 'Starter',
    user_type: 'Starter',
    full_name: 'Local user',
    company: '',
    address: '',
    country_code: '',
    current_period: null,
    authorized_features: {},
    expired_screen_type: null,
    tax_id: '',
    has_sso: false,
    team_display_name: '',
    is_subscribed_to_marketing_emails: false,
  },
  device: undefined,
  devices: undefined,
  team_members: [],
  team_invites: [],
  hasAccessToSharedData: null,
  access_objects: [],
  hasDowngradedToStarter: false,
  enterprise: null,
});

function projectLocalState(state) {
  if (!state || typeof state !== 'object') throw new TypeError('local account expected an object Redux state');
  if (state.user === LOCAL_USER && state.authorized === true) return state;
  return { ...state, user: LOCAL_USER, authorized: true };
}

function wrapReducer(rootReducer) {
  if (typeof rootReducer !== 'function') throw new TypeError('local account requires a Redux reducer');
  return function localAccountReducer(state, action) {
    // Replicated actions can reach a reducer without passing through that
    // process's middleware. Do not let a logout/reset reducer destroy any
    // slice before restoring the local projection.
    if (state !== undefined && action && BLOCKED_ACTION_TYPES.has(action.type)) {
      return projectLocalState(state);
    }
    return projectLocalState(rootReducer(state, action));
  };
}

const middleware = () => next => action => {
  if (action && BLOCKED_ACTION_TYPES.has(action.type)) return action;
  return next(action);
};

const protectedApis = new WeakSet();

function replaceWithNoop(target, name) {
  if (!target || (typeof target !== 'object' && typeof target !== 'function')) return;
  if (typeof target[name] !== 'function') return;
  Object.defineProperty(target, name, {
    configurable: true,
    writable: true,
    value: async function localAccountProtectedOperation() { return undefined; },
  });
}

function protectApi(api) {
  if (!api || (typeof api !== 'object' && typeof api !== 'function')) {
    throw new TypeError('local account requires an API facade');
  }
  if (protectedApis.has(api)) return api;
  protectedApis.add(api);

  // These facade methods can erase the whole local database/settings or the
  // offline identity. Per-entity store deletion used by imports is untouched.
  for (const name of ['logout', 'clearUserData', 'clearDB', 'deleteAccount']) {
    replaceWithNoop(api, name);
  }
  replaceWithNoop(api.auth, 'logout');
  // Also guard the lower-level account client in case a private saga bypasses
  // the facade. This does not affect local entity CRUD.
  replaceWithNoop(api.api, 'logout');
  replaceWithNoop(api.api, 'deleteAccount');
  return api;
}

function restore({ store, setAuthorized } = {}) {
  if (!store || typeof store.dispatch !== 'function' || typeof store.getState !== 'function') {
    throw new TypeError('local account requires a Redux store');
  }
  // SET_AUTHORIZED is UI-only Redux state.  The supplied callback must not set
  // API credentials; in the bundle it dispatches SET_AUTHORIZED(true).
  if (typeof setAuthorized === 'function') setAuthorized();
  store.dispatch({ type: LOCAL_ACCOUNT_ACTION });
  if (store.getState().user !== LOCAL_USER) throw new Error('local account reducer hook is not installed');
  return LOCAL_USER;
}

module.exports = {
  LOCAL_ACCOUNT_ACTION,
  LOCAL_USER,
  BLOCKED_ACTION_TYPES,
  wrapReducer,
  middleware,
  protectApi,
  restore,
};
