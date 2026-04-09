# Req 13: Settings Page

## User Story

**As a user, I want a unified settings page to manage my preferences, so that I can personalize my experience and control privacy and security options.**

---

## Requirements

### 13.1 Audio/Video Settings

**User Story:** As a user, I want to configure default audio/video devices and parameters, so that my preferred settings are automatically applied during calls.

#### Acceptance Criteria

1. WHEN the user enters the settings page THEN the system SHALL display the audio/video settings module
2. WHEN the user adjusts the default speaker volume THEN the system SHALL preview volume changes in real-time and persist settings to localStorage
3. WHEN the user adjusts the default microphone volume THEN the system SHALL display real-time microphone level feedback and persist settings
4. WHEN the user selects a default camera device THEN the system SHALL display all available camera devices in a dropdown list (obtained via `navigator.mediaDevices.enumerateDevices()`) and save the user's selection
5. WHEN the user selects a default microphone device THEN the system SHALL display all available microphone devices in a dropdown list and save the user's selection
6. WHEN the user selects a video quality preference (Auto/HD 720P/Save Data 360P) THEN the system SHALL save the preference and apply corresponding video encoding parameters (`RTCRtpEncodingParameters`) during subsequent call establishment
7. IF the user has not manually configured audio/video devices THEN the system SHALL automatically select system default devices during the first call

---

### 13.2 Appearance Settings

**User Story:** As a user, I want to customize the application's appearance, so that I can have a comfortable user experience.

#### Acceptance Criteria

1. WHEN the user enters the appearance settings module THEN the system SHALL display theme switching, language switching, and font size adjustment options
2. WHEN the user switches the theme (System/Light/Dark) THEN the system SHALL immediately apply the new theme style and persist the preference to localStorage
   - **System**: Monitor `prefers-color-scheme` media query changes
   - **Light**: Force apply light theme
   - **Dark**: Force apply dark theme
3. WHEN the user switches the interface language (Chinese/English) THEN the system SHALL immediately update all UI text (using leptos-i18n) without page refresh and persist the preference
4. WHEN the user adjusts the font size (Small/Medium/Large) THEN the system SHALL immediately update the global font size (via CSS variable `--font-size-base`) and persist the preference
   - **Small**: `--font-size-base: 14px`
   - **Medium**: `--font-size-base: 16px` (default)
   - **Large**: `--font-size-base: 18px`
5. IF the user visits for the first time without a language preference set THEN the system SHALL automatically select the corresponding language based on `navigator.language` (zh-CN or en, other languages default to en)

---

### 13.3 Privacy & Security Settings

**User Story:** As a user, I want to control my privacy settings and security options, so that I can protect my personal information and communication security.

#### Acceptance Criteria

1. WHEN the user enters the privacy & security settings module THEN the system SHALL display blacklist management, online status visibility, read receipts toggle, and other options
2. WHEN the user clicks on blacklist management THEN the system SHALL display the current blacklist (username, avatar, block time) and provide an "Unblock" button
3. WHEN the user unblocks a blacklisted user THEN the system SHALL immediately remove the user from the blacklist and allow both parties to re-establish connection invitations
4. WHEN the user switches online status visibility (Online/Invisible) THEN the system SHALL immediately update the user's online status broadcast behavior
   - **Online**: Broadcast online status to all users normally
   - **Invisible**: Do not broadcast online status to any users, but remain visible within rooms
5. WHEN the user toggles the read receipts switch THEN the system SHALL control whether to send read receipts to message senders
   - **On**: Send read receipts normally
   - **Off**: Do not send read receipts; the sender sees message status as "Delivered" instead of "Read"
6. WHEN the user switches blacklist or privacy settings THEN the system SHALL persist settings to local storage and maintain them after re-login

---

### 13.4 Notification Settings

**User Story:** As a user, I want to control notification behavior, so that I can receive reminders at appropriate times without being disturbed.

#### Acceptance Criteria

1. WHEN the user enters the notification settings module THEN the system SHALL display message notifications, call notifications, and do-not-disturb period configuration options
2. WHEN the user toggles the message notification switch THEN the system SHALL immediately apply the setting and persist it
   - **On**: Trigger browser notifications when receiving new messages (requires user authorization)
   - **Off**: Do not trigger message notifications
3. WHEN the user toggles the call notification switch THEN the system SHALL immediately apply the setting and persist it
   - **On**: Trigger browser notifications and ringtone when receiving incoming calls
   - **Off**: Do not trigger notifications for incoming calls; only display call prompts within the UI
4. WHEN the user sets a do-not-disturb period (start time, end time) THEN the system SHALL automatically mute all non-critical notifications (messages, calls) during that period
5. WHEN the current time is within the do-not-disturb period THEN the system SHALL display a "Do Not Disturb mode is enabled" banner at the top of the settings page
6. IF the browser has not granted notification permission THEN the system SHALL display a "Request Notification Permission" button in the notification settings module; clicking it calls `Notification.requestPermission()`
7. WHEN the user grants or denies notification permission THEN the system SHALL update the permission status display in the notification settings module

---

### 13.5 Data Management

**User Story:** As a user, I want to manage local data storage, so that I can control disk usage and backup important data.

#### Acceptance Criteria

1. WHEN the user enters the data management module THEN the system SHALL display clear chat history, clear cache, and export data options
2. WHEN the user clicks "Clear Chat History" THEN the system SHALL display a confirmation dialog listing the content scope to be deleted (current session messages / all local messages)
3. WHEN the user confirms clearing chat history THEN the system SHALL clear message data in IndexedDB and refresh the current chat interface
4. WHEN the user clicks "Clear Cache" THEN the system SHALL display the current cache size (including: Sticker resource cache, avatar cache, i18n resource cache) and provide a "Clear" button
5. WHEN the user confirms clearing cache THEN the system SHALL clear Service Worker cache and non-critical data in localStorage, while preserving user preference settings
6. WHEN the user clicks "Export Data" THEN the system SHALL support exporting in the following formats:
   - **JSON format**: Export all chat history, contact list, blacklist (structured data)
   - **HTML format**: Export chat history as readable HTML pages (including timestamps, senders, message content)
7. WHEN the user selects an export format and confirms THEN the system SHALL generate the corresponding file and trigger browser download; the filename includes the export date (e.g., `chat-export-2026-04-08.json`)
8. WHEN exported data contains sensitive information (such as encryption keys) THEN the system SHALL prompt the user to keep the exported file secure to avoid leakage

---

### 13.6 Settings Page UI & Interaction

**User Story:** As a user, I want the settings page layout to be clear and easy to operate, so that I can quickly find and adjust configuration items.

#### Acceptance Criteria

1. WHEN the user enters the settings page from the main interface THEN the system SHALL use a sidebar or drawer layout to display the settings module list
2. WHEN the user clicks a settings module (e.g., "Audio/Video Settings") THEN the system SHALL display detailed configuration items for that module in the right content area
3. WHEN a setting item changes THEN the system SHALL immediately save it and display "Saved" visual feedback (e.g., Toast notification or icon flash)
4. WHEN the user presses the `Escape` key on the settings page THEN the system SHALL close the settings page and return to the previous page
5. WHEN a setting item requires browser permission (e.g., notification permission, media device permission) THEN the system SHALL display the current permission status next to the setting item (Granted/Denied) and provide a quick action button
6. WHEN the user accesses the settings page on a mobile device THEN the system SHALL use a full-screen drawer layout to ensure configuration items are easy to operate on small screens
7. WHEN the settings page loads THEN the system SHALL read all saved preference settings from localStorage and apply them to the corresponding configuration item controls

---

## Relationship with Other Requirements

- **Req 3 (Audio/Video Calling)**: Audio/video settings default devices and quality preferences will affect parameter configuration during call establishment
- **Req 7 (UI & Theme)**: Appearance settings theme switching feature extends Req 7.5, providing a clear UI entry point
- **Req 10 (User Discovery)**: Blacklist management feature integrates with Req 10.15-22 blacklist mechanism, providing a unified management interface
- **Req 11 (Auth & Session)**: Online status visibility setting affects user status broadcast behavior after login
- **Req 12 (Message Persistence)**: Data management module's clear chat history feature directly operates IndexedDB message data

---

## Technical Implementation Constraints

1. **Persistent Storage**: All user preference settings SHALL use localStorage for storage, with key names using a unified prefix `settings_` (e.g., `settings_theme`, `settings_volume`)
2. **Reactive Updates**: Setting changes SHALL trigger global state updates through Leptos Signals mechanism, ensuring all related UI components refresh synchronously
3. **Permission Checks**: Setting items involving browser permissions (notifications, media devices) SHALL check permission status on page load and update UI when permissions change
4. **Internationalization**: All text content in the settings page SHALL support leptos-i18n bilingual switching
5. **Accessibility**: All settings controls SHALL comply with WCAG 2.1 AA standards, supporting keyboard navigation and screen readers
