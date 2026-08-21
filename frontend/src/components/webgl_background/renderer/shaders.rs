//! GLSL shader sources for the three WebGL background effects.
//!
//! Kept separate from [`super`]'s GL plumbing (context setup, RAF
//! loop, uniform uploads) so each file stays focused: this one is
//! pure shader text, the parent is pure Rust/`web_sys` glue.

/// Shared fullscreen-triangle vertex shader — every fragment shader
/// below is driven by this single VAO (see
/// `super::fullscreen_triangle_vao`).
pub const VERT: &str = r#"#version 300 es
in vec2 position;
out vec2 vUv;
void main() {
  vUv = position * 0.5 + 0.5;
  gl_Position = vec4(position, 0.0, 1.0);
}
"#;

/// Gradient Waves — raymarched plasma heightfield.
pub const FRAG_WAVES: &str = r#"#version 300 es
precision highp float;

uniform vec2  iResolution;
uniform float iTime;
uniform vec2  uMouse;
uniform vec3  uHorizonColor;
uniform vec3  uWaveColor;
uniform vec3  uCrestColor;
uniform float uOpacity;
// User-configurable knobs (Settings → Background → Gradient Waves).
// Every uniform below is a plain multiplier/offset around the
// original hard-coded shader constants (see the inline comment at
// each use site for which constant it replaces), so the defaults in
// `settings::background_waves::WaveConfig` reproduce the original,
// pre-configurable look exactly.
uniform float uWaveScale;
uniform float uWaveRatio;
uniform float uWaveSpeed;
uniform float uWaveSwell;
uniform float uWaveTurbulence;
uniform float uWaveTilt;
uniform float uWaveZoom;
uniform float uWaveHorizonHeight;
uniform float uWaveFogDepth;
uniform float uWaveBrightness;

out vec4 fragColor;

const float MAX_DIST = 20000.0;

float hash21(vec2 p) {
  vec3 p3 = fract(vec3(p.xyx) * 0.1031);
  p3 += dot(p3, p3.yzx + 33.33);
  return fract((p3.x + p3.y) * p3.z);
}

float plasma(vec3 r, vec2 freq, vec4 tc, float swell, float turbulence, float horizonHeight) {
  float mx = r.x + tc.x;
  // `35.0`/`20.0` warp magnitude scaled by `turbulence` (0 = perfectly
  // regular sine waves, 1 = original organic look, >1 = exaggerated).
  mx += turbulence * 35.0 * sin((r.y + mx) / 20.0 + tc.y);
  float my = r.y - tc.z;
  my += turbulence * 20.0 * cos(r.x / 23.0 + tc.w);
  // `2.5`/`2.5` amplitude terms scaled by `swell`; `5.5` baseline
  // offset shifted by `horizonHeight`.
  return r.z - (sin(mx * freq.x) * 2.5 * swell + sin(my * freq.y) * 2.5 * swell + 5.5 + horizonHeight);
}

float raymarch(vec3 pos, vec3 dir, vec2 freq, vec4 tc, float swell, float turbulence, float horizonHeight) {
  float dist = 0.0;
  for (int i = 0; i < 70; i++) {
    float dscene = plasma(pos + dist * dir, freq, tc, swell, turbulence, horizonHeight);
    if (abs(dscene) < 0.1) break;
    dist += 0.9 * dscene;
    if (!(abs(dist) < MAX_DIST)) return MAX_DIST;
  }
  return dist;
}

void main() {
  float T = iTime * 0.4 * uWaveSpeed;
  // `xDenom` is a fixed reference (matches the original hard-coded
  // `7.0`); `yDenom` is derived from `uWaveRatio` so the default
  // ratio (7/3) reproduces the original `3.0` exactly, while moving
  // the slider stretches/compresses the y-frequency independently
  // of x.
  float xDenom = 7.0;
  float yDenom = xDenom / max(uWaveRatio, 0.001);
  vec2 freq = vec2(0.6 * uWaveScale / xDenom, (0.6 * uWaveScale * 0.9) / yDenom);
  vec4 tc = vec4(T / 0.130, T / 0.810, T / 0.200, T / 0.710);

  // `vfov` (field of view) divided by `uWaveZoom`: >1 zooms in
  // (narrower FOV, terrain looks closer), <1 zooms out.
  float vfov = (3.14159 / 2.3) / max(uWaveZoom, 0.001);
  vec3 cam = vec3(0.0, 0.0, 30.0);

  vec2 uv = (gl_FragCoord.xy / iResolution.xy) - 0.5;
  uv.x *= iResolution.x / iResolution.y;
  uv.y *= -1.0;

  vec3 dir = vec3(0.0, 0.0, -1.0);
  float ulen = length(uv);
  float xrot = vfov * ulen;
  float c = cos(xrot); float s = sin(xrot);
  dir = mat3(1.0,0.0,0.0, 0.0,c,-s, 0.0,s,c) * dir;

  vec2 nuv = ulen > 1e-5 ? uv / ulen : vec2(1.0, 0.0);
  c = nuv.x; s = nuv.y;
  dir = mat3(c,-s,0.0, s,c,0.0, 0.0,0.0,1.0) * dir;

  // Camera pitch — was the hard-coded `1.11` radian constant, now
  // driven by `uWaveTilt`.
  c = cos(uWaveTilt); s = sin(uWaveTilt);
  dir = mat3(c,0.0,s, 0.0,1.0,0.0, -s,0.0,c) * dir;

  // Mouse parallax
  float yaw   = (uMouse.x - 0.5) * 0.5 * 0.4;
  float pitch = (uMouse.y - 0.5) * 0.5 * 0.4;
  c = cos(yaw);   s = sin(yaw);
  dir = mat3(c,0.0,s, 0.0,1.0,0.0, -s,0.0,c) * dir;
  c = cos(pitch); s = sin(pitch);
  dir = mat3(1.0,0.0,0.0, 0.0,c,-s, 0.0,s,c) * dir;

  float dist = raymarch(cam, dir, freq, tc, uWaveSwell, uWaveTurbulence, uWaveHorizonHeight);
  vec3 pos = cam + dist * dir;

  // `t` blends horizon → terrain color based on how close the ray hit
  // was. The raymarch typically converges around dist≈26-30 for most
  // screen directions (verified numerically), so a reference distance
  // of 15 made `t` — and therefore alpha — cap out around 0.5-0.6,
  // which combined with uOpacity produced barely-visible pixels
  // (measured maxAlpha≈38/255 via gl.readPixels). The original fix
  // raised the reference to 32; `uWaveFogDepth` now exposes that
  // reference distance directly (smaller = fog sooner / denser,
  // larger = detail reaches farther before fading).
  float t = clamp(uWaveFogDepth / max(dist, 0.001), 0.0, 1.0);
  vec3 body = mix(uWaveColor, uCrestColor, clamp(pos.z * 0.08 + 0.5, 0.0, 1.0));
  vec3 col = mix(uHorizonColor, body, t);
  col = clamp(col * uWaveBrightness, 0.0, 1.0);

  float alpha = clamp(t, 0.0, 1.0) * uOpacity;

  // Grain
  float g = hash21(gl_FragCoord.xy + mod(iTime, 64.0) * 11.0);
  alpha += (g - 0.5) * 0.05;
  alpha = clamp(alpha, 0.0, 1.0);

  // Straight (non-premultiplied) alpha — the blend func
  // (SRC_ALPHA, ONE_MINUS_SRC_ALPHA) applies the multiplication.
  // Pre-multiplying here would double-apply alpha and make the
  // waves nearly invisible at low opacity.
  fragColor = vec4(col, alpha);
}
"#;

/// Light Rays — volumetric beams from top anchor.
pub const FRAG_RAYS: &str = r#"#version 300 es
precision highp float;

uniform vec2  iResolution;
uniform float iTime;
uniform vec2  uMouse;
uniform vec3  uRaysColor;
uniform float uOpacity;

out vec4 fragColor;

float rayStrength(vec2 raySource, vec2 rayRefDirection, vec2 coord, float seedA, float seedB, float speed) {
  vec2 sourceToCoord = coord - raySource;
  vec2 dirNorm = normalize(sourceToCoord);
  float cosAngle = dot(dirNorm, rayRefDirection);
  // Softer power (was 1.0) so the beams stay bright across a wider
  // angular spread instead of collapsing to near-zero away from the
  // exact center axis — the original exponent made `spreadFactor`
  // one of four multiplicative dampeners that combined to <0.1
  // effective strength almost everywhere (measured alpha ≈ 0.06-0.1,
  // i.e. invisible against the page).
  float spreadFactor = pow(max(cosAngle, 0.0), 0.5);
  float distance = length(sourceToCoord);
  float maxDistance = iResolution.x * 2.0;
  float lengthFalloff = clamp((maxDistance - distance) / maxDistance, 0.0, 1.0);
  // Raised floor (was 0.5) — most on-screen pixels sit past
  // `iResolution.x` from the source, so the un-clamped formula was
  // pinned at its floor for the majority of the canvas.
  float fadeFalloff = clamp((iResolution.x * 1.0 - distance) / (iResolution.x * 1.0), 0.65, 1.0);
  // Raised amplitude/center (was 0.45/0.15 + 0.3/0.2) so the beams
  // read as bright bands instead of a faint shimmer.
  float baseStrength = clamp(
    (0.6 + 0.2 * sin(cosAngle * seedA + iTime * speed)) +
    (0.4 + 0.25 * cos(-cosAngle * seedB + iTime * speed)),
    0.0, 1.0
  );
  return baseStrength * lengthFalloff * fadeFalloff * spreadFactor;
}

void main() {
  vec2 coord = vec2(gl_FragCoord.x, iResolution.y - gl_FragCoord.y);
  vec2 rayPos = vec2(0.5 * iResolution.x, -0.2 * iResolution.y);
  vec2 rayDir = vec2(0.0, 1.0);

  // Mouse influence on ray direction
  vec2 mouseScreenPos = uMouse * iResolution.xy;
  vec2 mouseDirection = normalize(mouseScreenPos - rayPos);
  rayDir = normalize(mix(rayDir, mouseDirection, 0.1));

  float rays1 = rayStrength(rayPos, rayDir, coord, 36.2214, 21.11349, 1.5);
  float rays2 = rayStrength(rayPos, rayDir, coord, 22.3991, 18.0234, 1.1);

  // Combined strength boosted ~2x (weights raised from 0.5/0.4 to
  // 0.8/0.6, plus an explicit 1.6x multiplier) to compensate for the
  // four compounding falloff factors in `rayStrength`. Verified this
  // is the dominant lever: without it, alpha never exceeds ~0.1
  // regardless of `uOpacity`.
  float strength = clamp((rays1 * 0.8 + rays2 * 0.6) * 1.6, 0.0, 1.0);

  vec4 col = vec4(strength);

  // Vertical brightness gradient
  float brightness = 1.0 - (coord.y / iResolution.y);
  col.x *= 0.3 + brightness * 0.7;
  col.y *= 0.5 + brightness * 0.5;
  col.z *= 0.7 + brightness * 0.3;

  col.rgb *= uRaysColor;
  col.a = strength * uOpacity;
  fragColor = col;
}
"#;

/// Particles — vertex shader driving point sprites.
pub const VERT_PARTICLES: &str = r#"#version 300 es
in vec2 aPos;
in float aSize;
in float aPhase;
uniform vec2 iResolution;
uniform float iTime;
// Device pixel ratio — `gl_PointSize` is specified in framebuffer
// (device) pixels, but `aPos`/`iResolution` are CSS pixels (see the
// coordinate-system note on the fragment side). Without this scale
// factor, a "7px" particle on a DPR=2 display only occupies ~3.5
// framebuffer pixels — exactly the "particles barely visible" bug
// confirmed via `gl.readPixels` connected-component analysis (real
// rendered spot size was 1-3px against a requested 3-7px).
uniform float uDpr;
out float vAlpha;

void main() {
  // Drift (in CSS-pixel space)
  vec2 pos = aPos;
  pos.x += sin(iTime * 0.1 + aPhase) * 20.0;
  pos.y += cos(iTime * 0.15 + aPhase * 1.3) * 15.0;

  // Wrap around
  pos = mod(pos + 10.0, iResolution + 20.0) - 10.0;

  // Convert CSS pixels to clip space
  vec2 clip = (pos / iResolution) * 2.0 - 1.0;
  clip.y = -clip.y;
  gl_Position = vec4(clip, 0.0, 1.0);
  gl_PointSize = aSize * uDpr;

  // Twinkle — base/amplitude raised (was 0.45/0.25, floor 0.2) so
  // the dimmest point in the cycle still clears the `smoothstep`
  // radial falloff's translucent outer ring instead of disappearing
  // into the backdrop for a third of each cycle.
  vAlpha = 0.7 + 0.3 * sin(iTime * 0.02 * (0.5 + aPhase));
}
"#;

/// Particles — fragment shader: radial-gradient point sprite falloff.
pub const FRAG_PARTICLES: &str = r#"#version 300 es
precision highp float;
in float vAlpha;
uniform vec3 uColor;
uniform float uOpacity;
out vec4 fragColor;

void main() {
  // Radial gradient falloff — narrowed from smoothstep(1.0, 0.0, d)
  // to smoothstep(1.0, 0.35, d): the wide falloff made ~1/3 of the
  // point's radius (and therefore most of its rendered area at
  // small sizes) fade to near-zero alpha, so only a tiny solid
  // "core" a few device pixels wide was ever actually opaque.
  vec2 uv = gl_PointCoord * 2.0 - 1.0;
  float d = length(uv);
  float alpha = smoothstep(1.0, 0.35, d) * vAlpha * uOpacity;
  // Straight alpha — blend func handles the multiplication.
  fragColor = vec4(uColor, alpha);
}
"#;
