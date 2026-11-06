"use strict";

// No HTTP fallback. This bridge exists only for bundled Tauri assets.
const invoke = window.__TAURI__?.core?.invoke;
const shell = document.querySelector("main.shell");

const screen = document.createElement("section");
screen.className = "ipc-auth-screen";
screen.setAttribute("role", "region");
screen.setAttribute("aria-label", "Authentification locale Pravda-Bot");
const card = document.createElement("div");
card.className = "ipc-auth-card";
const heading = document.createElement("h1");
heading.textContent = "PRAVDA / MARKET";
const notice = document.createElement("p");
notice.textContent = "Authentification locale requise";
const form = document.createElement("form");
const passwordLabel = document.createElement("label");
passwordLabel.textContent = "Mot de passe";
const password = document.createElement("input");
password.type = "password";
password.required = true;
password.autocomplete = "current-password";
passwordLabel.append(password);
const confirmLabel = document.createElement("label");
confirmLabel.textContent = "Confirmer le mot de passe";
const confirmation = document.createElement("input");
confirmation.type = "password";
confirmation.autocomplete = "new-password";
confirmLabel.append(confirmation);
const errorText = document.createElement("p");
errorText.className = "ipc-auth-error";
errorText.setAttribute("role", "alert");
const submit = document.createElement("button");
submit.type = "submit";
submit.textContent = "Se connecter";
form.append(passwordLabel, confirmLabel, errorText, submit);
card.append(heading, notice, form);
screen.append(card);
document.body.append(screen);
shell.hidden = true;

let setup = false;
let dashboardLoaded = false;
let dashboardLoading = false;
let working = false;
let loginVisible = false;
// Capture only a closed diagnostic category and an in-file line number.
// Never copy an exception message, stack, full URL, user text or credentials.
// A script "load" event does not prove dashboard initialization succeeded.
let dashboardFaultCode = "DASHBOARD_INIT_UNKNOWN";
let dashboardFaultCause = "NOT_CLASSIFIED";
const dashboardSource = source =>
    typeof source === "string" &&
    /(?:^|\/)dashboard\.js(?:[?#].*)?$/.test(source);
window.addEventListener("error", event => {
    if (!dashboardSource(event.filename)) return;
    const name = event.error?.name;
    const category = name === "SyntaxError" ? "SYNTAX" :
        name === "ReferenceError" ? "REFERENCE" :
        name === "TypeError" ? "TYPE" :
        name === "RangeError" ? "RANGE" : "RUNTIME";
    const line = Number.isSafeInteger(event.lineno) && event.lineno > 0 &&
        event.lineno <= 100000 ? event.lineno : 0;
    dashboardFaultCode = "DASHBOARD_" + category + "_L" + line;
    // Only emit a closed, non-sensitive category. Never persist exception text
    // or a stack trace: runtime messages may embed user/system details.
    const reason = typeof event.error?.message === "string" ? event.error.message : "";
    dashboardFaultCause = category !== "RANGE" ? "NOT_RANGE" :
        /maximum call stack|too much recursion|stack overflow/i.test(reason) ? "STACK_OVERFLOW" :
        /invalid array length/i.test(reason) ? "ARRAY_LENGTH" :
        /invalid time value|invalid date/i.test(reason) ? "INVALID_DATE" :
        /fraction digits|precision/i.test(reason) ? "PRECISION" :
        /invalid count value|repeat count/i.test(reason) ? "REPEAT_COUNT" : "OTHER_RANGE";
}, true);

function showDashboardStartupFailure(code) {
    // The Rust session and stored password verifier are never modified here.
    // Do not keep an incompletely initialized dashboard interactive.
    shell.hidden = true;
    screen.hidden = false;
    form.hidden = true;
    notice.textContent = "Le tableau de bord n'a pas pu démarrer.";
    screen.querySelector(".ipc-startup-recovery")?.remove();

    const recovery = document.createElement("div");
    recovery.className = "ipc-startup-recovery";
    const explanation = document.createElement("p");
    explanation.setAttribute("role", "alert");
    explanation.textContent = "Échec de l'interface locale. Le mot de passe peut déjà avoir été configuré ; aucune donnée utilisateur n'a été effacée. Copiez le code ci-dessous pour le diagnostic.";
    const diagnostic = document.createElement("input");
    diagnostic.type = "text";
    diagnostic.readOnly = true;
    diagnostic.setAttribute("aria-label", "Code de diagnostic du tableau de bord");
    diagnostic.value = "PRAVDA-UI-STARTUP|" + code;
    diagnostic.addEventListener("focus", () => diagnostic.select());
    // Only an allowlisted static boot phase is exposed, not error text, stack,
    // URLs, system paths or saved credentials.
    const bootPhase = document.createElement("input");
    bootPhase.className = "ipc-bootstrap-phase";
    bootPhase.type = "text";
    bootPhase.readOnly = true;
    bootPhase.setAttribute("aria-label", "Phase de démarrage du tableau de bord");
    const phase = window.pravdaDashboardPhase;
    bootPhase.value = "PRAVDA-UI-PHASE|" +
        (typeof phase === "string" && /^P(?:0[0-9]|1[0-5]|02[A-I])$/.test(phase) ? phase : "PXX");
    bootPhase.addEventListener("focus", () => bootPhase.select());
    const safeCause = document.createElement("input");
    safeCause.className = "ipc-startup-cause";
    safeCause.type = "text";
    safeCause.readOnly = true;
    safeCause.setAttribute("aria-label", "Catégorie de l'erreur de démarrage");
    safeCause.value = "PRAVDA-UI-CAUSE|" + dashboardFaultCause;
    safeCause.addEventListener("focus", () => safeCause.select());
    const reload = document.createElement("button");
    reload.type = "button";
    reload.textContent = "Réessayer le chargement local";
    reload.addEventListener("click", () => window.location.reload());
    recovery.append(explanation, diagnostic, bootPhase, safeCause, reload);
    card.append(recovery);
}

// UI-only epoch for ignoring stale IPC results after logout or a new login.
// The real authorization boundary remains the Rust-owned session.
window.pravdaAuthGeneration = 0;

window.pravdaInvoke = async (command, args = {}) => {
    if (typeof invoke !== "function") throw new Error("TAURI_IPC_UNAVAILABLE");
    return invoke(command, args);
};

function showLogin() {
    // Idempotent: background polling must never erase a password being typed.
    if (loginVisible) return;
    window.pravdaAuthGeneration++;
    loginVisible = true;
    if (typeof window.pravdaLockRevolutConsent === "function") window.pravdaLockRevolutConsent();
    if (typeof window.pravdaClearPaper === "function") window.pravdaClearPaper();
    if (typeof window.pravdaClearAcceptance === "function") window.pravdaClearAcceptance();
    if (typeof window.pravdaClearUpdateReadiness === "function") window.pravdaClearUpdateReadiness();
    if (typeof window.pravdaClearMarketSession === "function") window.pravdaClearMarketSession();
    shell.hidden = true;
    screen.hidden = false;
    form.reset();
    errorText.textContent = "";
    password.focus();
}

window.pravdaRequireLogin = showLogin;

function activateDashboard() {
    window.pravdaAuthGeneration++;
    loginVisible = false;
    // Never display prior-session synthetic positions while the new snapshot loads.
    if (typeof window.pravdaClearPaper === "function") window.pravdaClearPaper();
    if (typeof window.pravdaClearAcceptance === "function") window.pravdaClearAcceptance();
    if (typeof window.pravdaClearUpdateReadiness === "function") window.pravdaClearUpdateReadiness();
    if (typeof window.pravdaClearMarketSession === "function") window.pravdaClearMarketSession();
    screen.hidden = true;
    shell.hidden = false;
    if (dashboardLoaded) {
        if (typeof window.refreshRevolutConsent === "function") window.refreshRevolutConsent();
        if (typeof window.refreshAll === "function") window.refreshAll();
        if (typeof window.refreshSecurity === "function") window.refreshSecurity();
        if (typeof window.refreshUpdateReadiness === "function") window.refreshUpdateReadiness();
        if (typeof window.refreshSecurityEvents === "function") window.refreshSecurityEvents();
        if (typeof window.refreshPaper === "function") window.refreshPaper();
        return;
    }
    // Multiple login confirmations must not inject dashboard.js twice.
    // Duplicate initialization can bind event handlers twice and repeat alerts.
    if (dashboardLoading) return;
    dashboardLoading = true;
    dashboardFaultCode = "DASHBOARD_INIT_UNKNOWN";
    dashboardFaultCause = "NOT_CLASSIFIED";
    window.pravdaDashboardReady = false;
    const script = document.createElement("script");
    script.src = "./dashboard.js";
    script.addEventListener("load", () => {
        dashboardLoading = false;
        const logoutForm = document.getElementById("ipc-logout-form");
        logoutForm.addEventListener("submit", async event => {
            event.preventDefault();
            try {
                await window.pravdaInvoke("ipc_auth_logout");
            } catch {
                // Revoked or expired sessions are already unauthenticated.
            }
            showLogin();
        });
        if (window.pravdaDashboardReady !== true) {
            showDashboardStartupFailure(dashboardFaultCode);
            return;
        }
        dashboardLoaded = true;
    });
    script.addEventListener("error", () => {
        dashboardLoading = false;
        showDashboardStartupFailure("DASHBOARD_RESOURCE_UNAVAILABLE");
    });
    document.body.append(script);
}

form.addEventListener("submit", async event => {
    event.preventDefault();
    if (working) return;
    working = true;
    submit.disabled = true;
    errorText.textContent = "";
    try {
        if (setup) {
            if (password.value !== confirmation.value) throw new Error("PASSWORD_MISMATCH");
            await window.pravdaInvoke("ipc_auth_setup", {
                password: password.value,
                confirm: confirmation.value
            });
            // Setup is one-shot. A later logout must offer login, not setup.
            setup = false;
            confirmLabel.hidden = true;
            confirmation.required = false;
            password.minLength = 0;
            password.autocomplete = "current-password";
            notice.textContent = "Saisissez votre mot de passe local.";
            submit.textContent = "Se connecter";
        } else {
            await window.pravdaInvoke("ipc_auth_login", { password: password.value });
        }
        form.reset();
        activateDashboard();
    } catch (error) {
        errorText.textContent = {
            PASSWORD_MISMATCH: "Les mots de passe ne correspondent pas.",
            WEAK_PASSWORD: "Utilisez au moins 15 caractères.",
            INVALID_CREDENTIALS: "Mot de passe incorrect.",
            RATE_LIMITED: "Trop de tentatives. Réessayez ultérieurement.",
            STORAGE_FAILURE: "Erreur de stockage local.",
            ALREADY_CONFIGURED: "Un mot de passe est déjà configuré."
        }[String(error)] || "Authentification indisponible.";
    } finally {
        working = false;
        submit.disabled = false;
    }
});

(async () => {
    if (typeof invoke !== "function") {
        errorText.textContent = "Pont Tauri IPC indisponible. Aucun repli HTTP.";
        submit.disabled = true;
        return;
    }
    try {
        setup = !(await window.pravdaInvoke("ipc_auth_status"));
        confirmLabel.hidden = !setup;
        confirmation.required = setup;
        password.minLength = setup ? 15 : 0;
        password.autocomplete = setup ? "new-password" : "current-password";
        notice.textContent = setup
            ? "Première utilisation : définissez un mot de passe local d'au moins 15 caractères."
            : "Saisissez votre mot de passe local.";
        submit.textContent = setup ? "Créer mon mot de passe" : "Se connecter";
        showLogin();
    } catch {
        errorText.textContent = "Initialisation IPC refusée. Vérifiez les capabilities de la fenêtre principale.";
        submit.disabled = true;
    }
})();
