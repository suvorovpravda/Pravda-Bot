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
let working = false;
let loginVisible = false;
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
    if (typeof window.pravdaClearMarketSession === "function") window.pravdaClearMarketSession();
    if (typeof window.pravdaClearUpdate === "function") window.pravdaClearUpdate();
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
    if (typeof window.pravdaClearMarketSession === "function") window.pravdaClearMarketSession();
    if (typeof window.pravdaClearUpdate === "function") window.pravdaClearUpdate();
    screen.hidden = true;
    shell.hidden = false;
    if (dashboardLoaded) {
        if (typeof window.refreshRevolutConsent === "function") window.refreshRevolutConsent();
        if (typeof window.refreshAll === "function") window.refreshAll();
        if (typeof window.refreshSecurity === "function") window.refreshSecurity();
        if (typeof window.refreshSecurityEvents === "function") window.refreshSecurityEvents();
        if (typeof window.refreshPaper === "function") window.refreshPaper();
        return;
    }
    const script = document.createElement("script");
    script.src = "./dashboard.js";
    script.addEventListener("load", () => {
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
            const banner = document.createElement("p");
            banner.className = "ipc-auth-error";
            banner.setAttribute("role", "alert");
            banner.textContent = "Interface non initialisée. Fermez puis rouvrez Pravda-Bot ; aucun mot de passe ni détail système n'a été enregistré.";
            shell.prepend(banner);
            return;
        }
        dashboardLoaded = true;
    });
    script.addEventListener("error", () => {
        shell.hidden = true;
        screen.hidden = false;
        errorText.textContent = "Impossible de charger les ressources locales.";
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
