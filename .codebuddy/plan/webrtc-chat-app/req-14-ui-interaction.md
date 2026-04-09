# Requirement 14: UI Interaction Design Specification

> Back to [Requirements Overview](./requirements.md)

**User Story:** As a user, I want a polished, intuitive, and consistent user interface with smooth interactions and clear visual feedback, so that I can enjoy a professional-grade user experience.

---

## 14.1 Page Layout Architecture

### 14.1.1 Main Application Shell

**User Story:** As a user, I want a consistent application layout across all pages, so that I can navigate efficiently without relearning the interface.

#### Acceptance Criteria

1. WHEN the application loads THEN the system SHALL display a persistent application shell consisting of:
   - **Sidebar** (width: 240-280px on desktop, collapsible on tablet/mobile)
   - **Main Content Area** (flexible width, minimum 320px)
   - **Optional Right Panel** (for user profiles, room details, width: 300-360px)

2. WHEN the user is on desktop (width ≥ 1024px) THEN the system SHALL display:
   - Left sidebar with: user profile card, search bar, room list, online users list
   - Main content area with: active chat/call/theater view
   - Right panel toggle button (visible when room details available)

3. WHEN the user is on tablet (768px ≤ width < 1024px) THEN the system SHALL:
   - Collapse sidebar to icon-only mode (48px width) by default
   - Display sidebar expand button at top-left
   - Auto-collapse right panel, accessible via floating button

4. WHEN the user is on mobile (width < 768px) THEN the system SHALL:
   - Hide sidebar completely, replaced by bottom navigation bar
   - Display hamburger menu button at top-left
   - Use full-screen drawer for sidebar content
   - Show main content in full width

5. WHEN the user resizes the browser window THEN the system SHALL:
   - Smoothly transition between layout modes without content loss
   - Preserve scroll position and UI state during transition
   - Complete layout transition within 300ms

---

### 14.1.2 Sidebar Layout

**User Story:** As a user, I want an organized sidebar that provides quick access to rooms, contacts, and settings.

#### Acceptance Criteria

1. WHEN the sidebar displays THEN the system SHALL show sections in the following order:
   - **User Profile Card** (top): Avatar, username, online status, settings button
   - **Search Bar**: Placeholder text "Search rooms or users...", search icon
   - **Room List Section**: Section header "Rooms" with "+" button for creating new room
   - **Online Users Section**: Section header "Online Users" with count badge

2. WHEN a room list item is displayed THEN the system SHALL show:
   - Room avatar (32x32px, rounded corners)
   - Room name (bold if unread messages exist)
   - Unread message count badge (red pill, positioned at right)
   - Last message preview (gray text, truncated to 1 line, max 30 characters)
   - Timestamp of last message (gray text, right-aligned)

3. WHEN the user hovers over a room list item THEN the system SHALL:
   - Highlight the item with a subtle background color change (theme-dependent)
   - Show a context menu button (three dots) at the right edge
   - Transition hover state within 150ms

4. WHEN the user clicks a room list item THEN the system SHALL:
   - Highlight the item with active background color
   - Load the room content in the main area
   - Clear the unread message badge
   - Move the item to the top of the room list (if sorted by recent activity)

---

## 14.2 Core Component Design

### 14.2.1 Chat Message Component

**User Story:** As a user, I want clearly structured chat messages with intuitive interactions, so that I can easily read and manage messages.

#### Acceptance Criteria

1. WHEN a text message is displayed THEN the system SHALL show:
   - Sender avatar (36x36px, circular)
   - Sender name (bold, theme-dependent color)
   - Message timestamp (gray, smaller font, positioned below sender name)
   - Message content (with proper Markdown rendering)
   - Message status indicator (sending → sent → delivered → read, positioned below message)

2. WHEN a message belongs to the current user THEN the system SHALL:
   - Right-align the message content
   - Use a different background color (theme-dependent, e.g., light blue in light theme)
   - Display message status indicators (✓ sending, ✓✓ sent, ✓✓ delivered, ✓✓ read in blue)

3. WHEN a message belongs to another user THEN the system SHALL:
   - Left-align the message content
   - Use default background color (theme-dependent, e.g., gray in light theme)
   - Not display message status indicators

4. WHEN the user hovers over a message THEN the system SHALL:
   - Show a reaction button (emoji face icon) at the top-right corner
   - Show a context menu button (three dots) next to the reaction button
   - Fade in hover controls within 150ms

5. WHEN the user clicks the reaction button THEN the system SHALL:
   - Display an emoji picker popup (6x8 grid, showing recent + commonly used emojis)
   - Position the popup anchored to the message, avoiding viewport overflow
   - Allow selecting multiple emojis to add as reactions

6. WHEN the user clicks the context menu button THEN the system SHALL display options:
   - "Reply" (for all messages)
   - "Quote" (for all messages, inserts quoted message reference in input)
   - "Copy Text" (for text messages)
   - "Revoke" (for own messages within 2 minutes, with confirmation dialog)
   - "Forward" (opens user/room selection modal)

7. WHEN a message has reactions THEN the system SHALL display:
   - Reaction pills below the message content
   - Each pill shows: emoji + count
   - Highlight the pill if the current user has added that reaction
   - Allow clicking a reaction pill to toggle the current user's reaction

8. WHEN a message is a forwarded message THEN the system SHALL display:
   - A "Forwarded" header above the message content, showing: forward icon (↪) + "Forwarded from {original_sender_name}"
   - The forwarded header SHALL use a smaller font size (Caption style, 12px) and secondary text color
   - The message content below the forwarded header SHALL render identically to the original message type (text, sticker, voice, image)
   - The forwarded message bubble SHALL have a slightly different background color (5% darker/lighter than regular messages) to visually distinguish it

9. WHEN a message contains a reply reference (`reply_to` field) THEN the system SHALL display:
   - A quoted message block above the message content, with:
     - Left accent border (4px width, primary color)
     - Original sender name (bold, Caption style)
     - Original message preview (truncated to 2 lines, max 100 characters, Body Small style, secondary text color)
   - The quoted block SHALL have a subtle background color (theme-dependent: `#F0F0F0` light / `#2A2A2A` dark)
   - The quoted block SHALL be clickable (cursor: pointer), with hover effect (darken background by 5%)
   - WHEN clicked THEN the system SHALL scroll to the original message and highlight it briefly (see Req 2.15d)
   - IF the original message was revoked THEN the quoted block SHALL display "Original message has been revoked" in italic, secondary text color

---

### 14.2.2 Voice Message Component

**User Story:** As a user, I want intuitive voice message controls with visual waveform feedback, so that I can easily record, play, and navigate voice messages.

#### Acceptance Criteria

1. WHEN a voice message is displayed THEN the system SHALL show:
   - Play/pause button (circular, 40px diameter, primary color)
   - Waveform visualization (60-80 bars, rendered via Canvas)
   - Current playback time (MM:SS format, positioned below waveform)
   - Total duration (MM:SS format, positioned at right end)
   - Playback progress indicator (colored overlay on waveform)

2. WHEN the user clicks the play button THEN the system SHALL:
   - Change the button icon to pause
   - Start waveform animation (bars pulse in sync with audio playback)
   - Update playback progress indicator smoothly (60fps)
   - Display a moving playhead (vertical line) on the waveform

3. WHEN the user clicks on the waveform THEN the system SHALL:
   - Seek to the corresponding playback position
   - Update the progress indicator immediately
   - Continue or pause playback based on current state

4. WHEN the user hovers over the waveform THEN the system SHALL:
   - Show a timestamp tooltip at the mouse position
   - Update tooltip content as the mouse moves along the waveform

5. WHEN playback completes THEN the system SHALL:
   - Change the play button icon back to play
   - Reset the progress indicator to the beginning
   - Stop waveform animation

---

### 14.2.3 Image Message Component

**User Story:** As a user, I want to view images with smooth loading and intuitive preview interactions, so that I can efficiently browse and inspect image messages.

#### Acceptance Criteria

1. WHEN an image message is displayed THEN the system SHALL:
   - Show a placeholder with aspect ratio preserved during loading
   - Display a loading spinner (circular, centered) while the image loads
   - Render the image with max-width: 320px, max-height: 480px, maintain aspect ratio
   - Apply border-radius: 8px to the image container

2. WHEN the image fails to load THEN the system SHALL:
   - Display a fallback placeholder with broken image icon
   - Show "Failed to load image" text below the icon
   - Provide a "Retry" button

3. WHEN the user clicks an image THEN the system SHALL:
   - Open a fullscreen image preview modal
   - Display the image centered, scaled to fit the viewport
   - Show a semi-transparent dark overlay behind the image
   - Add a close button (X icon) at the top-right corner
   - Enable zoom controls (+ / - / reset buttons)

4. WHEN the user is in image preview mode THEN the system SHALL:
   - Support keyboard shortcuts: `Escape` to close, `+` / `-` to zoom, `0` to reset zoom
   - Support mouse wheel for zooming (scroll up = zoom in, scroll down = zoom out)
   - Support pinch-to-zoom on touch devices
   - Display image filename and size at the bottom

5. WHEN multiple images are in a single message THEN the system SHALL:
   - Display a grid layout (2x2 for 4 images, 2x3 for 6 images, etc.)
   - Show a "+N" overlay if more than 6 images (clicking opens full gallery)
   - Allow left/right arrow navigation in preview mode

---

### 14.2.4 File Transfer Component

**User Story:** As a user, I want clear file transfer progress indicators with intuitive controls, so that I can monitor and manage file transfers efficiently.

#### Acceptance Criteria

1. WHEN a file transfer message is displayed THEN the system SHALL show:
   - File icon (based on file type: document, image, video, archive, etc.)
   - Filename (truncated to 30 characters with ellipsis if too long)
   - File size (human-readable: KB, MB, GB)
   - Transfer status (pending, transferring, completed, failed)
   - Transfer progress bar (for outgoing and incoming transfers)

2. WHEN a file is being transferred THEN the system SHALL:
   - Display a progress bar with percentage label (e.g., "45%")
   - Show transfer speed (e.g., "2.3 MB/s")
   - Show estimated time remaining (e.g., "12 seconds remaining")
   - Update progress smoothly (not jumping in large increments)

3. WHEN the user hovers over a transferring file THEN the system SHALL:
   - Show a "Pause" button (pause icon)
   - Show a "Cancel" button (X icon, red color)
   - Fade in buttons within 150ms

4. WHEN the user clicks the pause button THEN the system SHALL:
   - Change the button icon to "Resume"
   - Pause the file transfer
   - Retain the current progress

5. WHEN the transfer completes successfully THEN the system SHALL:
   - Change the file icon to a success indicator (green checkmark)
   - Replace progress bar with "Open" button
   - Show "Download complete" text briefly (2 seconds), then fade out

6. WHEN the transfer fails THEN the system SHALL:
   - Change the file icon to an error indicator (red X)
   - Replace progress bar with "Retry" button
   - Show error message (e.g., "Connection lost", "File corrupted")

---

### 14.2.5 Sticker Picker Component

**User Story:** As a user, I want an organized sticker picker with efficient search and preview, so that I can quickly find and send the perfect sticker.

#### Acceptance Criteria

1. WHEN the user opens the sticker picker THEN the system SHALL display:
   - Search bar at the top (placeholder: "Search stickers...")
   - Sticker pack tabs below the search bar (horizontal scrollable)
   - Sticker grid (4-5 columns, scrollable vertically)
   - Frequently used stickers section at the top

2. WHEN a sticker pack tab is selected THEN the system SHALL:
   - Highlight the tab with an underline or background color
   - Load stickers for that pack (lazy load if not cached)
   - Scroll the grid to the top of the selected pack's stickers

3. WHEN the user types in the search bar THEN the system SHALL:
   - Filter stickers by keyword (matching sticker name or tags)
   - Display matching stickers in real-time (debounce 300ms)
   - Show "No stickers found" message if no matches

4. WHEN the user hovers over a sticker THEN the system SHALL:
   - Scale up the sticker slightly (scale: 1.1, transition: 150ms)
   - Show the sticker name in a tooltip
   - Add a subtle shadow effect

5. WHEN the user clicks a sticker THEN the system SHALL:
   - Send the sticker immediately
   - Close the sticker picker
   - Add the sticker to the "Frequently used" section

6. WHEN the sticker picker is open THEN the system SHALL:
   - Allow closing by clicking outside the picker
   - Allow closing by pressing `Escape` key
   - Trap focus within the picker (for accessibility)

---

## 14.3 Interaction Flows & Transitions

### 14.3.1 Room Navigation Flow

**User Story:** As a user, I want smooth transitions when navigating between rooms, so that I maintain context and orientation.

#### Acceptance Criteria

1. WHEN the user switches from one room to another THEN the system SHALL:
   - Fade out the current room content (150ms)
   - Clear the message list
   - Load the new room messages from IndexedDB (show skeleton loading if slow)
   - Fade in the new room content (150ms)
   - Total transition time SHALL NOT exceed 400ms

2. WHEN the user navigates to a room with many messages (>100) THEN the system SHALL:
   - Display a skeleton message list (3-5 placeholder items) during loading
   - Load messages in batches (50 messages per batch)
   - Scroll to the bottom of the message list after initial load
   - Allow the user to scroll up to load older messages (infinite scroll)

3. WHEN the user returns to a previously visited room THEN the system SHALL:
   - Restore scroll position to the last seen location
   - Highlight new messages that arrived since the last visit (with a "New messages" divider)
   - Scroll to the first new message if the user was at the bottom before leaving

---

### 14.3.2 Modal & Dialog Animations

**User Story:** As a user, I want smooth modal animations that provide clear visual hierarchy, so that I can focus on the current task.

#### Acceptance Criteria

1. WHEN a modal opens THEN the system SHALL:
   - Fade in a semi-transparent overlay (opacity 0 → 0.5, 200ms)
   - Scale up the modal from 0.9 to 1.0 opacity (200ms, ease-out)
   - Trap focus within the modal (for accessibility)
   - Disable background scrolling

2. WHEN a modal closes THEN the system SHALL:
   - Fade out the overlay (opacity 0.5 → 0, 150ms)
   - Scale down the modal from 1.0 to 0.9 opacity (150ms, ease-in)
   - Restore focus to the element that triggered the modal
   - Re-enable background scrolling

3. WHEN a confirmation dialog appears THEN the system SHALL:
   - Display a warning icon (for destructive actions like delete)
   - Show a clear message explaining the action and its consequences
   - Provide "Cancel" and "Confirm" buttons (Confirm styled as primary or danger based on action severity)
   - Default focus on the "Cancel" button (to prevent accidental confirmation)

4. WHEN a toast notification appears THEN the system SHALL:
   - Slide in from the top-right corner (300ms, ease-out)
   - Display for 3 seconds (auto-dismiss)
   - Allow manual dismiss by clicking an X button
   - Stack multiple toasts vertically (newer on top)
   - Slide out to the right when dismissed (200ms, ease-in)

---

### 14.3.3 Call Interface Transitions

**User Story:** As a user, I want smooth transitions when entering and exiting call interfaces, so that I can seamlessly switch between chat and call modes.

#### Acceptance Criteria

1. WHEN an incoming call arrives THEN the system SHALL:
   - Display a fullscreen call notification overlay (slide up from bottom, 400ms)
   - Show caller avatar, name, and call type (audio/video)
   - Provide "Accept" (green) and "Decline" (red) buttons
   - Play ringtone (if enabled in settings)
   - Vibrate device (if on mobile and enabled)

2. WHEN the user accepts a call THEN the system SHALL:
   - Transition to the call interface with a cross-fade animation (300ms)
   - Display the local video preview in a small PiP window (bottom-right corner)
   - Show remote participant videos in the main area
   - Fade in call control buttons (mute, camera, screen share, hang up) at the bottom

3. WHEN the user ends a call THEN the system SHALL:
   - Fade out the call interface (200ms)
   - Return to the previous view (chat room or room list)
   - Display a "Call ended" toast with duration (e.g., "Call ended • 12:34")
   - Clear video streams and release media devices

4. WHEN a participant joins or leaves an ongoing call THEN the system SHALL:
   - Smoothly re-layout the video grid (animated transition, 300ms)
   - Fade in the new participant's video (if joining)
   - Fade out the leaving participant's video (if leaving)
   - Update the participant count indicator

---

## 14.4 Micro-Interactions & Feedback

### 14.4.1 Button Interactions

**User Story:** As a user, I want responsive button feedback, so that I can feel confident about my interactions.

#### Acceptance Criteria

1. WHEN the user hovers over a primary button THEN the system SHALL:
   - Lighten the background color by 10% (for light theme) or darken by 10% (for dark theme)
   - Change the cursor to `pointer`
   - Apply a subtle shadow (elevation increase)
   - Transition all changes within 150ms

2. WHEN the user presses a button THEN the system SHALL:
   - Scale down the button slightly (scale: 0.98)
   - Darken the background color further (10% darker than hover state)
   - Apply the pressed state immediately (no transition delay)

3. WHEN the user releases a button after pressing THEN the system SHALL:
   - Scale the button back to normal (1.0)
   - Restore the hover state (if mouse is still over the button)
   - Trigger the button's action

4. WHEN a button is disabled THEN the system SHALL:
   - Reduce opacity to 50%
   - Change cursor to `not-allowed`
   - Remove hover and pressed effects
   - Display a tooltip explaining why the button is disabled (on hover)

---

### 14.4.2 Input Field Interactions

**User Story:** As a user, I want clear input field feedback, so that I can confidently enter and submit information.

#### Acceptance Criteria

1. WHEN an input field is focused THEN the system SHALL:
   - Highlight the border with the primary color (2px width)
   - Display a subtle glow effect around the border (box-shadow)
   - Hide the placeholder text
   - Show a blinking cursor at the current input position

2. WHEN the user types in an input field THEN the system SHALL:
   - Update the value in real-time
   - Show a character count if there is a max length limit
   - Display validation feedback immediately (if validation fails)

3. WHEN validation fails for an input field THEN the system SHALL:
   - Change the border color to error color (red)
   - Shake the input field briefly (horizontal shake animation, 300ms)
   - Display an error message below the input field (fade in, 150ms)
   - Prevent form submission until validation passes

4. WHEN validation passes for a previously invalid field THEN the system SHALL:
   - Change the border color to success color (green) briefly (1 second)
   - Fade out the error message
   - Restore the normal border color after 1 second

---

### 14.4.3 Loading States

**User Story:** As a user, I want clear loading indicators, so that I understand the system is working and can estimate wait times.

#### Acceptance Criteria

1. WHEN content is loading THEN the system SHALL display one of the following:
   - **Skeleton loading**: Gray placeholder rectangles with pulse animation (for lists, cards)
   - **Spinner**: Circular indeterminate spinner (for quick operations, <2 seconds)
   - **Progress bar**: Determinate progress bar with percentage (for file transfers, uploads)
   - **Skeleton + text**: Skeleton placeholder with "Loading..." text (for slow operations, >2 seconds)

2. WHEN a skeleton loader is displayed THEN the system SHALL:
   - Mimic the shape and layout of the actual content
   - Apply a shimmer effect (gradient moving from left to right, 1.5s loop)
   - Use theme-appropriate gray tones

3. WHEN a spinner is displayed THEN the system SHALL:
   - Use the primary color for the spinner
   - Rotate smoothly at a constant speed (1s per rotation)
   - Center the spinner in the loading area

4. WHEN a long-loading operation (>5 seconds) THEN the system SHALL:
   - Display an estimated time remaining (if available)
   - Provide a "Cancel" button (if the operation is cancellable)
   - Show a reassuring message (e.g., "This may take a while...")

---

### 14.4.4 Empty States

**User Story:** As a user, I want helpful empty state illustrations and messages, so that I understand what to do next when there's no content.

#### Acceptance Criteria

1. WHEN the message list is empty (new room) THEN the system SHALL display:
   - A friendly illustration (e.g., speech bubbles, wave emoji)
   - Message: "No messages yet. Say hello! 👋"
   - A quick action: "Send a sticker" button

2. WHEN the search results are empty THEN the system SHALL display:
   - An illustration (e.g., magnifying glass, confused emoji)
   - Message: "No results found for '[search query]'"
   - Suggestions: "Try different keywords" or "Clear filters"

3. WHEN the room list is empty THEN the system SHALL display:
   - An illustration (e.g., empty room, plus icon)
   - Message: "You haven't joined any rooms yet"
   - Quick actions: "Create a room" and "Join a room" buttons

4. WHEN the online users list is empty THEN the system SHALL display:
   - An illustration (e.g., sleeping emoji, moon)
   - Message: "No one is online right now"
   - Subtext: "Check back later or invite friends!"

---

## 14.5 Gesture & Touch Interactions

### 14.5.1 Mobile Touch Gestures

**User Story:** As a mobile user, I want intuitive touch gestures, so that I can efficiently navigate and interact with the application.

#### Acceptance Criteria

1. WHEN the user swipes right on a message (mobile) THEN the system SHALL:
   - Reveal a "Reply" action button (slide from left edge)
   - Allow the user to tap the reply button to start a reply
   - Cancel the swipe if the user releases before reaching the action threshold

2. WHEN the user swipes left on a message (mobile) THEN the system SHALL:
   - Reveal "More" actions (edit, delete, forward, etc.)
   - Display actions in a horizontal action bar
   - Allow the user to tap an action to execute it

3. WHEN the user pulls down on the message list (mobile) THEN the system SHALL:
   - Trigger a pull-to-refresh animation
   - Load new messages from IndexedDB and server (if online)
   - Show a refresh indicator during loading
   - Snap back to the top after refreshing

4. WHEN the user long-presses a message (mobile) THEN the system SHALL:
   - Display a context menu (similar to right-click on desktop)
   - Haptic feedback (vibrate briefly)
   - Highlight the selected message

5. WHEN the user pinch-zooms an image (mobile) THEN the system SHALL:
   - Scale the image smoothly following the pinch gesture
   - Constrain zoom to a reasonable range (0.5x to 3x)
   - Allow panning the image when zoomed in

6. WHEN the user double-taps an image (mobile) THEN the system SHALL:
   - Zoom in to 2x (if currently at 1x)
   - Zoom out to 1x (if currently at 2x or higher)
   - Animate the zoom transition smoothly (300ms)

---

### 14.5.2 Keyboard Navigation

**User Story:** As a keyboard user, I want comprehensive keyboard shortcuts, so that I can navigate and interact efficiently without a mouse.

#### Acceptance Criteria

1. WHEN the user presses `Tab` THEN the system SHALL:
   - Move focus to the next focusable element
   - Follow a logical tab order: sidebar → main content → right panel (if open)
   - Display a visible focus indicator (outline) on the focused element

2. WHEN the user presses `Shift + Tab` THEN the system SHALL:
   - Move focus to the previous focusable element

3. WHEN the user presses `Enter` or `Space` on a focused button THEN the system SHALL:
   - Activate the button (same as clicking with mouse)

4. WHEN the user presses `Escape` THEN the system SHALL:
   - Close the topmost modal or popup (if any are open)
   - Cancel any in-progress actions (e.g., message edit, sticker picker)
   - Return focus to the previous context

5. WHEN the user presses arrow keys (`↑`, `↓`) in a list THEN the system SHALL:
   - Move focus to the previous/next list item
   - Scroll the list to keep the focused item visible

6. WHEN the user presses arrow keys (`←`, `→`) in a horizontal list THEN the system SHALL:
   - Move focus to the previous/next item

7. WHEN the user presses `Ctrl/Cmd + K` THEN the system SHALL:
   - Open a quick search modal (command palette style)
   - Allow searching for rooms, users, or commands
   - Display recent searches and suggestions

8. WHEN the user presses `Ctrl/Cmd + N` THEN the system SHALL:
   - Open the "Create Room" modal

9. WHEN the user is in a chat room THEN the following shortcuts SHALL work:
   - `↑` (in empty input field): Edit previous message sent by the user
   - `Ctrl/Cmd + Shift + M`: Toggle mute in call
   - `Ctrl/Cmd + Shift + V`: Toggle video in call

---

## 14.6 Animation & Motion Design Principles

### 14.6.1 Animation Timing Standards

**User Story:** As a user, I want consistent and smooth animations, so that the application feels polished and professional.

#### Acceptance Criteria

1. WHEN implementing micro-interactions (hover, press, toggle) THEN the system SHALL use:
   - Duration: 100-200ms
   - Easing: `ease-out` or `ease-in-out`

2. WHEN implementing state transitions (modal open/close, page transitions) THEN the system SHALL use:
   - Duration: 200-400ms
   - Easing: `cubic-bezier(0.4, 0, 0.2, 1)` (Material Design standard easing)

3. WHEN implementing attention-grabbing animations (new message arrival, notification) THEN the system SHALL use:
   - Duration: 300-500ms
   - Easing: `spring` physics-based animation (for natural feel)

4. WHEN implementing continuous animations (loading spinners, waveform visualizations) THEN the system SHALL:
   - Use `requestAnimationFrame` for smooth 60fps animation
   - Pause animations when the tab is not visible (to save resources)
   - Use CSS animations where possible (for better performance)

5. WHEN implementing drag-and-drop interactions THEN the system SHALL:
   - Use `transform` properties for positioning (GPU-accelerated)
   - Apply `will-change: transform` to animated elements
   - Avoid animating `width`, `height`, `top`, `left` directly (causes layout thrashing)

---

### 14.6.2 Accessibility & Reduced Motion

**User Story:** As a user with motion sensitivity, I want the option to reduce or disable animations, so that I can use the application comfortably.

#### Acceptance Criteria

1. WHEN the user has enabled "Reduce Motion" in system settings (`prefers-reduced-motion: reduce`) THEN the system SHALL:
   - Disable all non-essential animations (decorative transitions, background animations)
   - Replace animations with instant transitions (0ms duration)
   - Keep essential animations (loading spinners, progress bars) but simplify them

2. WHEN the user manually enables "Reduced Motion" in the settings page THEN the system SHALL:
   - Apply the same behavior as system-level "Reduce Motion"
   - Persist the preference to localStorage

3. WHEN animations are disabled THEN the system SHALL:
   - Ensure all functionality remains accessible
   - Provide alternative visual feedback (e.g., instant color changes, bold text)
   - Test with real users who have motion sensitivity

---

## 14.7 Theming & Visual Design Tokens

### 14.7.1 Theme Switching Behavior

**User Story:** As a user, I want the application to automatically adapt to my system's theme and allow manual switching, so that I can enjoy a personalized visual experience.

#### Acceptance Criteria

1. WHEN the system detects the OS dark mode preference (`prefers-color-scheme: dark`) THEN the system SHALL automatically switch to dark theme
2. WHEN a user manually switches theme THEN the system SHALL:
   - Immediately apply the new theme (no page refresh needed)
   - Persist the preference to localStorage
   - Override the system preference until the user selects "Auto" mode
3. IF the user has not manually set a theme preference THEN the system SHALL follow the system theme automatically
4. WHEN theme switches THEN the system SHALL implement smooth transition animation via CSS variables (transition), avoiding flicker (transition duration: 200ms)
5. WHEN the user visits the app for the first time THEN the system SHALL:
   - Check localStorage for saved theme preference
   - If no preference found, detect system theme via `prefers-color-scheme` media query
   - Apply the detected theme automatically
6. WHEN the user selects "Auto" mode in settings THEN the system SHALL:
   - Remove manual theme preference from localStorage
   - Re-enable system theme detection
   - Immediately switch to match current system theme

---

### 14.7.2 Color System

**User Story:** As a user, I want a consistent color system that adapts to themes, so that the application feels cohesive and accessible.

#### Acceptance Criteria

1. WHEN the application renders THEN the system SHALL use a design token system for all colors:
   - **Primary**: Main brand color (used for buttons, links, active states)
   - **Secondary**: Supporting color (used for icons, secondary buttons)
   - **Background**: Surface colors (background, cards, modals)
   - **Text**: Text colors (primary, secondary, disabled, inverse)
   - **Status**: Semantic colors (success, warning, error, info)
   - **Interactive**: Hover, pressed, focused states

2. WHEN the theme is light THEN the system SHALL use:
   - Background: `#FFFFFF` (main), `#F5F5F5` (sidebar), `#E8E8E8` (hover)
   - Primary: `#2196F3` (blue)
   - Text: `#212121` (primary), `#757575` (secondary)
   - Success: `#4CAF50`, Warning: `#FF9800`, Error: `#F44336`, Info: `#2196F3`

3. WHEN the theme is dark THEN the system SHALL use:
   - Background: `#121212` (main), `#1E1E1E` (sidebar), `#2C2C2C` (hover)
   - Primary: `#64B5F6` (lighter blue for contrast)
   - Text: `#FFFFFF` (primary), `#B0B0B0` (secondary)
   - Success: `#66BB6A`, Warning: `#FFA726`, Error: `#EF5350`, Info: `#64B5F6`

4. WHEN applying colors THEN the system SHALL:
   - Ensure all text meets WCAG 2.1 AA contrast ratio (4.5:1 for normal text, 3:1 for large text)
   - Never rely on color alone to convey information (use icons, text, or patterns as backup)
   - Use CSS variables for all colors (enable theme switching without re-render)

---

### 14.7.3 Typography System

**User Story:** As a user, I want a consistent typography system, so that text is readable and hierarchically clear.

#### Acceptance Criteria

1. WHEN the application renders text THEN the system SHALL use a type scale:
   - **Heading 1**: 32px / 40px line-height / 700 font-weight
   - **Heading 2**: 24px / 32px line-height / 700 font-weight
   - **Heading 3**: 20px / 28px line-height / 600 font-weight
   - **Body**: 16px / 24px line-height / 400 font-weight (default)
   - **Body Small**: 14px / 20px line-height / 400 font-weight
   - **Caption**: 12px / 16px line-height / 400 font-weight

2. WHEN rendering usernames, room names THEN the system SHALL:
   - Use Heading 3 style
   - Truncate with ellipsis if too long (max 30 characters)
   - Never wrap to multiple lines

3. WHEN rendering message content THEN the system SHALL:
   - Use Body style
   - Support multi-line text
   - Preserve line breaks and whitespace (pre-wrap)

4. WHEN rendering timestamps THEN the system SHALL:
   - Use Caption style
   - Use monospace font for time display (optional, for alignment)

5. WHEN the user adjusts font size in settings THEN the system SHALL:
   - Scale all typography proportionally (Small: 0.875x, Medium: 1x, Large: 1.125x)
   - Update CSS variable `--font-size-base` globally

---

### 14.7.4 Spacing & Layout Grid

**User Story:** As a user, I want consistent spacing and alignment, so that the interface feels organized and balanced.

#### Acceptance Criteria

1. WHEN the application lays out elements THEN the system SHALL use an 8px grid system:
   - **4px**: Extra small spacing (icon gaps, tight groups)
   - **8px**: Small spacing (component internal padding)
   - **16px**: Medium spacing (between related elements)
   - **24px**: Large spacing (section separators)
   - **32px**: Extra large spacing (major layout sections)

2. WHEN rendering message list items THEN the system SHALL:
   - Apply 8px vertical padding per message
   - Apply 16px horizontal padding for message content

3. WHEN rendering sidebar items THEN the system SHALL:
   - Apply 8px vertical padding per item
   - Apply 16px horizontal padding for content

4. WHEN rendering modals THEN the system SHALL:
   - Apply 24px padding around modal content
   - Apply 16px gap between modal elements

---

## 14.8 Icon System

### 14.8.1 Icon Library & Technology

**User Story:** As a developer, I want a consistent and maintainable icon system, so that the application has a unified visual language and optimal performance.

#### Acceptance Criteria

1. WHEN implementing icons THEN the system SHALL use **leptos-icons** as the primary icon library:
   - Use leptos-icons which provides type-safe, tree-shakable icon components for Leptos
   - Support popular icon sets: Heroicons, Lucide, Feather Icons, Tabler Icons
   - Enable tree-shaking to include only used icons in the final WASM bundle

2. WHEN importing icons THEN the system SHALL:
   - Import individual icon components (not entire icon sets) to minimize bundle size
   - Use consistent naming convention: `Icon{IconName}` (e.g., `IconSend`, `IconUser`, `IconSettings`)
   - Wrap frequently used icons in custom components for reusability

3. WHEN the icon library is configured THEN the system SHALL:
   - Define a central icon registry module (`src/components/icons/mod.rs`) for all icon imports
   - Export custom icon components with predefined styles (size, color, stroke-width)
   - Provide icon component variants: `Icon{Name}Small` (16px), `Icon{Name}Medium` (24px), `Icon{Name}Large` (32px)

4. WHEN selecting icons for the application THEN the system SHALL:
   - Use **Lucide Icons** as the default icon set (modern, clean, consistent)
   - Use **Heroicons** for specific UI elements if needed (alternative style)
   - Ensure all icons have consistent visual weight and style
   - Avoid mixing multiple icon sets in the same context (e.g., same page, same component)

---

### 14.8.2 Icon Usage Guidelines

**User Story:** As a user, I want consistent and clear iconography, so that I can quickly understand functionality without reading text.

#### Acceptance Criteria

1. WHEN displaying functional icons THEN the system SHALL use the following standard icons:
   - **Navigation**: `IconHome`, `IconMessageCircle`, `IconUsers`, `IconSettings`, `IconSearch`
   - **Chat Actions**: `IconSend`, `IconPaperclip`, `IconMic`, `IconImage`, `IconSmile`, `IconSticker`
   - **Media Controls**: `IconPlay`, `IconPause`, `IconVolume2`, `IconVolumeX`, `IconVideo`, `IconVideoOff`, `IconPhone`, `IconPhoneOff`
   - **File Actions**: `IconDownload`, `IconUpload`, `IconFile`, `IconFileText`, `IconFileImage`, `IconFileVideo`, `IconFileAudio`, `IconArchive`, `IconX` (close/delete)
   - **Status Indicators**: `IconCheck`, `IconCheckCheck` (delivered/read), `IconClock` (pending), `IconAlertCircle` (error), `IconWifi` (connected), `IconWifiOff` (disconnected)
   - **User Status**: `IconCircle` (online, filled green), `IconCircle` (offline, empty gray), `IconMoon` (away)
   - **Room Actions**: `IconPlus` (create), `IconLogIn` (join), `IconLogOut` (leave), `IconLock` (password protected), `IconUnlock` (no password)
   - **Settings**: `IconUser`, `IconBell`, `IconShield`, `IconPalette`, `IconDatabase`, `IconGlobe`
   - **Theater**: `IconFilm`, `IconMessageSquare` (danmaku), `IconMaximize`, `IconMinimize`

2. WHEN displaying icons in buttons THEN the system SHALL:
   - Use 20px icons for small buttons (height: 28-32px)
   - Use 24px icons for medium buttons (height: 36-40px)
   - Use 28px icons for large buttons (height: 44-48px)
   - Maintain 8px gap between icon and text (if both present)

3. WHEN displaying icons in list items THEN the system SHALL:
   - Use 20px icons for sidebar navigation items
   - Use 16px icons for inline indicators (e.g., status badges, file type icons)
   - Vertically center icons with adjacent text

4. WHEN displaying standalone decorative icons THEN the system SHALL:
   - Use 32px icons for empty state illustrations
   - Use 48px icons for large decorative elements (e.g., welcome screens)
   - Apply reduced opacity (40-60%) for decorative icons to avoid visual clutter

---

### 14.8.3 Icon Styling & Theming

**User Story:** As a user, I want icons that adapt to the current theme and context, so that the interface remains visually cohesive.

#### Acceptance Criteria

1. WHEN the theme changes THEN the system SHALL:
   - Update icon colors via CSS variables (`--icon-color-primary`, `--icon-color-secondary`, `--icon-color-disabled`)
   - In light theme: primary icons use `#212121`, secondary icons use `#757575`
   - In dark theme: primary icons use `#FFFFFF`, secondary icons use `#B0B0B0`

2. WHEN styling icons THEN the system SHALL:
   - Use `stroke-width: 2` for all outline-style icons (Lucide, Feather)
   - Use `stroke-width: 1.5` for dense UI areas (mobile, compact lists)
   - Apply `currentColor` for icon stroke color to inherit from parent text color (for seamless theming)

3. WHEN an icon is interactive (clickable) THEN the system SHALL:
   - Apply hover effect: lighten color by 10% (light theme) or brighten by 10% (dark theme)
   - Apply pressed effect: darken color by 10% (light theme) or dim by 10% (dark theme)
   - Display cursor: pointer
   - Provide visual feedback within 150ms transition

4. WHEN an icon indicates status (success, error, warning) THEN the system SHALL:
   - Use semantic colors: Success (`#4CAF50` light / `#66BB6A` dark), Error (`#F44336` light / `#EF5350` dark), Warning (`#FF9800` light / `#FFA726` dark)
   - Pair icons with text labels for accessibility (screen readers)
   - Never rely on icon color alone to convey meaning

---

### 14.8.4 Icon Accessibility

**User Story:** As a user with visual impairments, I want icons to be accessible via screen readers and keyboard navigation, so that I can understand their purpose.

#### Acceptance Criteria

1. WHEN an icon is purely decorative THEN the system SHALL:
   - Add `aria-hidden="true"` attribute to hide from screen readers
   - Not provide `alt` text or `aria-label`

2. WHEN an icon conveys meaning or is interactive THEN the system SHALL:
   - Provide `aria-label` attribute describing the icon's function (e.g., `aria-label="Send message"`)
   - If icon is inside a button, apply `aria-label` to the button element (not the icon itself)
   - Ensure icon labels are concise (max 3-5 words)

3. WHEN an icon button has no visible text THEN the system SHALL:
   - Always provide `aria-label` on the button element
   - Example: `<button aria-label="Settings"><IconSettings /></button>`

4. WHEN icons are used in lists or menus THEN the system SHALL:
   - Ensure icons are keyboard-focusable (if interactive)
   - Provide visible focus indicator (outline) when focused
   - Support `Enter` / `Space` to activate icon buttons

5. WHEN icons animate (e.g., loading spinner) THEN the system SHALL:
   - Add `aria-busy="true"` attribute during animation
   - Add `aria-label="Loading"` for loading spinners
   - Stop animation when content is loaded or operation completes

---

### 14.8.5 Icon Performance & Bundle Optimization

**User Story:** As a user, I want fast-loading pages, so that I can use the application without delay.

#### Acceptance Criteria

1. WHEN building the application THEN the system SHALL:
   - Enable tree-shaking for leptos-icons (only include used icons in WASM bundle)
   - Monitor icon-related bundle size (target: < 20KB gzipped for all icons)
   - Avoid importing entire icon sets (use individual icon imports)

2. WHEN using icons THEN the system SHALL:
   - Reuse icon component instances where possible (avoid re-importing)
   - Prefer icon components over inline SVG (for consistency and maintainability)
   - Lazy-load rarely used icons (e.g., icons only used in modals or settings)

3. WHEN rendering icons THEN the system SHALL:
   - Use SVG icons (via leptos-icons) instead of icon fonts (better performance, no font loading)
   - Avoid animating icon `width` or `height` properties (animate `transform: scale()` instead)
   - Use `will-change: transform` for animated icons (to hint GPU acceleration)

4. WHEN testing icon usage THEN the system SHALL:
   - Verify all icons are correctly imported and rendered
   - Check for missing icon fallbacks (display placeholder if icon fails to load)
   - Ensure icon colors are correct in both light and dark themes

---

## 14.9 Responsive Design & Layout Modes

**User Story:** As a user, I want the application to have a good experience on different devices, so that I can use it comfortably regardless of my screen size.

### 14.9.1 Layout Mode Definitions

#### Acceptance Criteria

1. WHEN a user accesses on a desktop browser (width ≥ 1024px) THEN the system SHALL display:
   - Full sidebar (240-280px width) + main content area dual-column layout
   - Optional right panel (300-360px width) when room details are available
   - All navigation elements visible in sidebar
   - Multi-column layouts for content grids

2. WHEN a user accesses on a tablet device (768px ≤ width < 1024px) THEN the system SHALL display:
   - Collapsible sidebar (collapsed to 48px icon-only mode by default)
   - Sidebar expand button at top-left
   - Auto-collapsed right panel (accessible via floating button)
   - 2-column layouts for content grids

3. WHEN a user accesses on a mobile device (width < 768px) THEN the system SHALL display:
   - Single-column layout
   - Sidebar hidden, replaced by bottom navigation bar
   - Hamburger menu button at top-left
   - Full-screen drawer for sidebar content (slides in from left)
   - Main content in full width
   - Touch targets increased to 44x44px minimum (for accessibility)

4. WHEN a video call is in progress on mobile (width < 768px) THEN the system SHALL:
   - Provide fullscreen mode automatically
   - Maximize the video display area
   - Hide sidebar and navigation elements
   - Show call controls in a floating overlay

---

### 14.9.2 Responsive Breakpoints

#### Acceptance Criteria

1. WHEN the viewport width is ≥ 1024px THEN the system SHALL:
   - Display full desktop layout (sidebar + main content + optional right panel)
   - Show all navigation elements in the sidebar
   - Use multi-column layouts for content grids

2. WHEN the viewport width is 768px - 1023px THEN the system SHALL:
   - Display tablet layout (collapsible sidebar + main content)
   - Collapse right panel by default (accessible via floating button)
   - Use 2-column layouts for content grids

3. WHEN the viewport width is < 768px THEN the system SHALL:
   - Display mobile layout (single column, bottom navigation)
   - Use full-screen drawers for sidebar content
   - Stack all elements vertically
   - Increase touch targets to 44x44px minimum (for accessibility)

4. WHEN the viewport height is < 500px (landscape mobile) THEN the system SHALL:
   - Collapse top/bottom bars to minimal height
   - Prioritize content area
   - Use horizontal tabs instead of vertical navigation

5. WHEN the user rotates the device THEN the system SHALL:
   - Smoothly transition to the new layout (within 300ms)
   - Preserve scroll position and UI state
   - Not reload the page or lose data

---

## 14.10 Network Quality Indicator

### 14.10.1 Network Quality Display

**User Story:** As a user in a call, I want to see a clear network quality indicator, so that I can understand the current connection quality and take action if needed.

#### Acceptance Criteria

1. WHEN a user is in an audio/video call THEN the system SHALL display a network quality indicator icon in the call interface:
   - The indicator SHALL be positioned at the top-right corner of each participant's video tile
   - The indicator SHALL use a signal strength icon with 4 bars (similar to mobile signal strength)
   - The indicator SHALL update every 5 seconds based on `RTCPeerConnection.getStats()` metrics

2. WHEN the network quality is assessed THEN the system SHALL classify it into 4 levels based on the following thresholds:
   - **Excellent** (4 bars, green): RTT < 100ms AND packet loss < 1%
   - **Good** (3 bars, green): RTT < 200ms AND packet loss < 3%
   - **Fair** (2 bars, yellow): RTT < 400ms AND packet loss < 8%
   - **Poor** (1 bar, red): RTT ≥ 400ms OR packet loss ≥ 8%

3. WHEN the user hovers over the network quality indicator THEN the system SHALL display a tooltip with detailed metrics:
   - Round-trip time (RTT): "{value} ms"
   - Packet loss rate: "{value}%"
   - Estimated bandwidth: "{value} Mbps"
   - Connection type: "Direct (P2P)" or "Relayed (TURN)"

4. WHEN the network quality drops to "Poor" THEN the system SHALL:
   - Display a brief toast notification: "Network quality is poor, video quality may be affected"
   - Add a pulsing animation to the indicator icon (to draw attention)
   - The toast SHALL appear at most once per 30 seconds (to avoid notification spam)

5. WHEN the network quality recovers from "Poor" to "Good" or "Excellent" THEN the system SHALL:
   - Update the indicator icon immediately
   - Display a brief toast: "Network quality restored" (auto-dismiss after 2 seconds)

6. WHEN the user is in a multi-user call THEN the system SHALL:
   - Display individual network quality indicators for each participant's connection
   - Show the user's own network quality indicator on their local video preview
   - Calculate the user's overall network quality as the worst quality among all peer connections

7. WHEN the user is NOT in a call (text chat only) THEN the system SHALL:
   - Display a small connection status icon in the sidebar header (connected/disconnected)
   - NOT display detailed network quality metrics (only relevant during calls)

---

## Relationship with Other Requirements

- **Req 7 (UI & Theme)**: This document extends Req 7 with detailed component designs, interaction patterns, and animation specifications
- **Req 2 (Chat)**: Chat message components (14.2.1-14.2.5) define the visual and interactive design for chat features
- **Req 3 (AV Call)**: Call interface transitions (14.3.3) define the animations for audio/video calling
- **Req 14 (Settings)**: Appearance settings in Req 14.2 allow users to customize theme and font size, which affects design tokens (14.7)
- **Req 13 (Theater)**: Theater mode UI will follow the same component and animation principles defined in this document
- **Icon System**: Section 14.8 defines the icon library (leptos-icons), usage guidelines, theming, accessibility, and performance optimization

---

## Technical Implementation Notes

> **CSS Architecture Constraint:** This project uses **native CSS only** — no third-party CSS frameworks (e.g., Tailwind CSS, Bootstrap, Bulma, Stylist, etc.) are permitted. All styling SHALL be implemented using modern native CSS features. This ensures zero external CSS dependencies, full control over the styling layer, and leverages the latest CSS specifications for maximum expressiveness and performance.

1. **Native CSS with Modern Features**: All styling SHALL use native CSS with the following modern features:
   - **CSS Custom Properties (Variables)**: All design tokens (colors, spacing, typography, shadows, border-radius) SHALL be defined as CSS custom properties on `:root` / `[data-theme]` selectors for theme switching
   - **CSS Nesting**: Use native CSS nesting (`&` selector) for component-scoped styles, reducing selector repetition and improving readability
   - **`@layer`**: Organize styles into cascade layers (`@layer reset, tokens, base, components, utilities`) for predictable specificity management
   - **`@container` Queries**: Use container queries for component-level responsive design (e.g., sidebar items adapting to sidebar width, message bubbles adapting to chat area width), in addition to `@media` queries for page-level layout
   - **`color-mix()` & `oklch()`**: Use `color-mix()` for hover/pressed state color derivation (e.g., `color-mix(in oklch, var(--color-primary) 90%, black)`) and `oklch()` for perceptually uniform color definitions
   - **`:has()` Selector**: Use `:has()` for parent-based conditional styling (e.g., `.message:has(.reaction-bar)` to adjust message padding when reactions are present)
   - **`@scope`**: Use `@scope` for component-level style encapsulation where CSS Modules are not used, preventing style leakage between components
   - **CSS Subgrid**: Use `subgrid` for aligning nested grid items (e.g., message list items with aligned avatars, names, and timestamps)
   - **`@starting-style`**: Use `@starting-style` for entry animations on dynamically inserted elements (e.g., new messages appearing in chat, toast notifications)
   - **Anchor Positioning (`anchor()`)**: Use CSS anchor positioning for tooltips, popovers, and context menus, replacing JavaScript-based positioning logic
   - **View Transitions API**: Use `document.startViewTransition()` for smooth page/view transitions (e.g., switching between rooms, opening settings)
   - **Scroll-driven Animations**: Use `animation-timeline: scroll()` for scroll-linked effects (e.g., parallax headers, progress indicators on long message lists)
2. **CSS File Organization**: Styles SHALL be organized as follows:
   - `/styles/tokens.css` — Design tokens (CSS custom properties for colors, spacing, typography, shadows)
   - `/styles/reset.css` — CSS reset / normalize
   - `/styles/base.css` — Base element styles (body, headings, links, forms)
   - `/styles/components/*.css` — Per-component styles (e.g., `message.css`, `sidebar.css`, `modal.css`)
   - `/styles/utilities.css` — Utility classes (if needed, kept minimal)
   - `/styles/main.css` — Entry point that imports all layers via `@layer` and `@import`
3. **Component Library**: Build custom accessible Leptos components (no equivalent headless UI library exists for Leptos/WASM yet); implement ARIA attributes and keyboard navigation directly in component code
4. **Animation**: Use CSS transitions/animations and `@starting-style` for simple interactions, `requestAnimationFrame` for complex animations (e.g., waveform, danmaku), View Transitions API for page-level transitions; avoid JavaScript-based animation libraries incompatible with WASM
5. **Icon System**: Use `leptos-icons` with Lucide icon set for consistent, tree-shakable, performant iconography
6. **Responsive Design**: Use CSS Grid (with Subgrid), Flexbox, `@container` queries, and `@media` queries for layouts; avoid fixed pixel widths; use `clamp()` for fluid typography and spacing
7. **Performance**: Use `will-change`, `transform`, and `opacity` for animations to leverage GPU acceleration; use `content-visibility: auto` for off-screen content optimization; use `@layer` to minimize specificity conflicts
8. **Testing**: Implement visual regression testing using Playwright screenshot comparison to catch unintended UI changes
