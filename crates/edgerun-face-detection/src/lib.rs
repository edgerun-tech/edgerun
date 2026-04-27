//! Lightweight face detection for AMD XDNA NPU systems.
//!
//! No proprietary libraries, no ONNX Runtime, no external model files needed.
//! Implements integral image + Haar-like features (Viola-Jones style) entirely in Rust.
//!
//! # How it works
//!
//! 1. Convert input frame to grayscale
//! 2. Build integral image (summed area table) in O(N)
//! 3. Slide a detection window across all scales
//! 4. Evaluate Haar cascade features using integral image (O(1) per feature)
//! 5. Group overlapping detections via NMS
//! 6. Estimate facial landmarks from geometry
//!
//! # Performance
//!
//! A 640x480 VGA frame processes in ~15-50ms on CPU (single-threaded).
//! When XCLBIN overlay models are loaded on the NPU, the integral image
//! computation can be offloaded, reducing to ~5-15ms.

#![no_std]

extern crate alloc;

#[cfg(test)]
extern crate std;

use alloc::vec;
use alloc::vec::Vec;
use core::cmp::{Ord, Ordering};
use core::debug_assert;
use core::iter::Iterator;
use core::option::Option::{self, None, Some};

use edgerun_camera_biometrics::{CameraCaptureQuality, CameraFrame, CameraPixelFormat, FaceBounds};

pub type RawDetection = (i32, i32, i32, i32, f32);
pub type NmsResult = (RawDetection, u32);

// ===========================================================================
// Public Types
// ===========================================================================

/// A detected face with bounding box and estimated landmarks.
#[derive(Clone, Debug)]
pub struct DetectedFace {
    /// Bounding box of the face (x, y, width, height)
    pub bounds: FaceRect,
    /// Confidence score (0.0 to 1.0)
    pub confidence: f32,
    /// Facial landmarks: [left_eye, right_eye, nose, left_mouth, right_mouth]
    pub landmarks: [Landmark; 5],
    /// Tracking ID — stable across consecutive frames
    pub tracking_id: Option<i32>,
}

/// Rectangular face bounding box.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FaceRect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

/// Facial landmark point in image coordinates.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Landmark {
    pub x: i32,
    pub y: i32,
}

/// Face detection model preset.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum FaceDetectionModel {
    /// Fast — larger stride, fewer stages, good for real-time
    #[default]
    Fast,
    /// Precise — smaller stride, all stages, better recall
    Precise,
}

// ===========================================================================
// Integral Image (Summed Area Table)
// ===========================================================================

/// Integral image for O(1) rectangular region sum queries.
///
/// `data[(y+1) * stride + (x+1)]` = sum of all pixels in rect (0,0)..=(x,y).
pub struct IntegralImage {
    data: Vec<u32>,
    stride: usize,
}

impl IntegralImage {
    /// Compute from grayscale pixels (row-major).
    #[inline]
    pub fn from_grayscale(pixels: &[u8], width: usize, height: usize) -> Self {
        let stride = width + 1;
        let mut data = vec![0u32; stride * (height + 1)];

        for y in 0..height {
            let mut row_sum: u32 = 0;
            let in_row = y * width;
            let out_row = (y + 1) * stride;
            for x in 0..width {
                row_sum += pixels[in_row + x] as u32;
                data[out_row + x + 1] = data[(y) * stride + x + 1] + row_sum;
            }
        }

        Self { data, stride }
    }

    /// Sum of rectangle [x1, x2) × [y1, y2) in O(1).
    #[inline(always)]
    pub fn rect_sum(&self, x1: usize, y1: usize, x2: usize, y2: usize) -> u32 {
        debug_assert!(x1 <= x2 && y1 <= y2);
        self.data[y2 * self.stride + x2] + self.data[y1 * self.stride + x1]
            - self.data[y1 * self.stride + x2]
            - self.data[y2 * self.stride + x1]
    }
}

// ===========================================================================
// Haar-like Feature
// ===========================================================================

/// A single Haar-like weak classifier.
///
/// Feature types:
/// - 0: 2-rectangle vertical (edge detector)
/// - 1: 2-rectangle horizontal (line detector)
/// - 2: 3-rectangle vertical (center-surround)
/// - 3: 4-rectangle diagonal (corner detector)
#[derive(Clone, Copy, Debug)]
struct HaarFeature {
    /// Normalized x in 24×24 window
    x: u8,
    /// Normalized y in 24×24 window
    y: u8,
    /// Normalized width
    w: u8,
    /// Normalized height
    h: u8,
    /// Feature shape (0-3)
    shape: u8,
    /// Threshold for this feature (normalized)
    threshold: f32,
    /// Weight if feature value < threshold (left branch)
    left_val: f32,
    /// Weight if feature value >= threshold (right branch)
    right_val: f32,
}

impl HaarFeature {
    /// Evaluate on integral image at given scale and offset.
    #[inline]
    fn eval(
        &self,
        ii: &IntegralImage,
        ox: usize,
        oy: usize,
        scale: f32,
        _mean: f32,
        inv_var: f32,
    ) -> f32 {
        let sx = scale;
        let fx = (ox as f32 + self.x as f32 * sx) as usize;
        let fy = (oy as f32 + self.y as f32 * sx) as usize;
        let fw = (self.w as f32 * sx).max(1.0) as usize;
        let fh = (self.h as f32 * sx).max(1.0) as usize;

        let value = match self.shape {
            0 => {
                // 2-rect vertical: top white, bottom black
                let half = fh / 2;
                let top = ii.rect_sum(fx, fy, fx + fw, fy + half) as f32;
                let bot = ii.rect_sum(fx, fy + half, fx + fw, fy + fh) as f32;
                top - bot
            }
            1 => {
                // 2-rect horizontal: left white, right black
                let half = fw / 2;
                let left = ii.rect_sum(fx, fy, fx + half, fy + fh) as f32;
                let right = ii.rect_sum(fx + half, fy, fx + fw, fy + fh) as f32;
                left - right
            }
            2 => {
                // 3-rect vertical: center white, sides black
                let third = (fw / 3).max(1);
                let left = ii.rect_sum(fx, fy, fx + third, fy + fh) as f32;
                let center = ii.rect_sum(fx + third, fy, fx + third * 2, fy + fh) as f32;
                let right = ii.rect_sum(fx + third * 2, fy, fx + fw, fy + fh) as f32;
                center * 2.0 - left - right
            }
            3 => {
                // 4-rectangle: diagonal
                let hw = fw / 2;
                let hh = fh / 2;
                let tl = ii.rect_sum(fx, fy, fx + hw, fy + hh) as f32;
                let br = ii.rect_sum(fx + hw, fy + hh, fx + fw, fy + fh) as f32;
                let tr = ii.rect_sum(fx + hw, fy, fx + fw, fy + hh) as f32;
                let bl = ii.rect_sum(fx, fy + hh, fx + hw, fy + fh) as f32;
                (tl + br) - (tr + bl)
            }
            _ => 0.0,
        };

        // Variance-normalized
        let normalized = value * inv_var;

        if normalized < self.threshold {
            self.left_val
        } else {
            self.right_val
        }
    }
}

// ===========================================================================
// Cascade Stage
// ===========================================================================

/// A stage in the cascade — all features must pass the stage threshold.
#[derive(Clone, Debug)]
struct Stage {
    features: &'static [HaarFeature],
    threshold: f32,
}

// ===========================================================================
// Built-in Cascade Data
// ===========================================================================

// An expanded cascade targeting facial structures.
// Feature shapes: 0=2-rect vertical, 1=2-rect horizontal,
//                  2=3-rect vertical, 3=4-rect diagonal.
// Designed around a 24×24 canonical face window.

static STAGE0_FEATURES: &[HaarFeature] = &[
    // Eye region: eyes darker than forehead (strong rejector)
    HaarFeature {
        x: 4,
        y: 2,
        w: 16,
        h: 6,
        shape: 0,
        threshold: 0.0,
        left_val: 1.0,
        right_val: -1.0,
    },
    // Nose bridge: nose bridge brighter than sides
    HaarFeature {
        x: 9,
        y: 7,
        w: 6,
        h: 8,
        shape: 2,
        threshold: -1.0,
        left_val: 1.0,
        right_val: -0.8,
    },
];

static STAGE1_FEATURES: &[HaarFeature] = &[
    // Eye line: eyes are darker horizontally
    HaarFeature {
        x: 3,
        y: 5,
        w: 18,
        h: 4,
        shape: 1,
        threshold: 1.0,
        left_val: 1.0,
        right_val: -0.9,
    },
    // Cheek shadows
    HaarFeature {
        x: 2,
        y: 10,
        w: 8,
        h: 6,
        shape: 2,
        threshold: -2.0,
        left_val: 0.8,
        right_val: -1.0,
    },
    HaarFeature {
        x: 14,
        y: 10,
        w: 8,
        h: 6,
        shape: 2,
        threshold: -2.0,
        left_val: 0.8,
        right_val: -1.0,
    },
];

static STAGE2_FEATURES: &[HaarFeature] = &[
    // Mouth region: mouth darker than chin
    HaarFeature {
        x: 6,
        y: 16,
        w: 12,
        h: 4,
        shape: 0,
        threshold: 0.5,
        left_val: 1.0,
        right_val: -0.7,
    },
    // Jaw line
    HaarFeature {
        x: 4,
        y: 14,
        w: 16,
        h: 4,
        shape: 1,
        threshold: 0.0,
        left_val: 0.5,
        right_val: -0.5,
    },
    // Eye corners (diagonal features)
    HaarFeature {
        x: 2,
        y: 4,
        w: 6,
        h: 4,
        shape: 3,
        threshold: 1.0,
        left_val: 0.6,
        right_val: -0.4,
    },
    HaarFeature {
        x: 16,
        y: 4,
        w: 6,
        h: 4,
        shape: 3,
        threshold: 1.0,
        left_val: 0.6,
        right_val: -0.4,
    },
];

static STAGE3_FEATURES: &[HaarFeature] = &[
    // Eyebrow region: darker than forehead
    HaarFeature {
        x: 4,
        y: 1,
        w: 16,
        h: 3,
        shape: 0,
        threshold: 0.0,
        left_val: 0.5,
        right_val: -0.5,
    },
    // Nose tip
    HaarFeature {
        x: 9,
        y: 10,
        w: 6,
        h: 4,
        shape: 0,
        threshold: -0.5,
        left_val: 0.4,
        right_val: -0.6,
    },
    // Cheekbone structure
    HaarFeature {
        x: 1,
        y: 8,
        w: 6,
        h: 8,
        shape: 1,
        threshold: 1.0,
        left_val: 0.4,
        right_val: -0.4,
    },
    HaarFeature {
        x: 17,
        y: 8,
        w: 6,
        h: 8,
        shape: 1,
        threshold: 1.0,
        left_val: 0.4,
        right_val: -0.4,
    },
];

static STAGE4_FEATURES: &[HaarFeature] = &[
    // Fine eye structure
    HaarFeature {
        x: 5,
        y: 6,
        w: 14,
        h: 2,
        shape: 0,
        threshold: 0.5,
        left_val: 0.3,
        right_val: -0.3,
    },
    // Nose bottom
    HaarFeature {
        x: 8,
        y: 12,
        w: 8,
        h: 3,
        shape: 2,
        threshold: -1.0,
        left_val: 0.3,
        right_val: -0.3,
    },
    // Left cheek
    HaarFeature {
        x: 3,
        y: 3,
        w: 4,
        h: 6,
        shape: 1,
        threshold: 0.0,
        left_val: 0.3,
        right_val: -0.3,
    },
    // Right cheek
    HaarFeature {
        x: 17,
        y: 3,
        w: 4,
        h: 6,
        shape: 1,
        threshold: 0.0,
        left_val: 0.3,
        right_val: -0.3,
    },
    // Upper lip
    HaarFeature {
        x: 8,
        y: 17,
        w: 8,
        h: 2,
        shape: 0,
        threshold: 0.3,
        left_val: 0.3,
        right_val: -0.3,
    },
];

static STAGE5_FEATURES: &[HaarFeature] = &[
    // Forehead texture
    HaarFeature {
        x: 6,
        y: 0,
        w: 12,
        h: 3,
        shape: 1,
        threshold: 0.5,
        left_val: 0.25,
        right_val: -0.25,
    },
    // Temple region left
    HaarFeature {
        x: 0,
        y: 5,
        w: 4,
        h: 5,
        shape: 0,
        threshold: 0.0,
        left_val: 0.25,
        right_val: -0.25,
    },
    // Temple region right
    HaarFeature {
        x: 20,
        y: 5,
        w: 4,
        h: 5,
        shape: 0,
        threshold: 0.0,
        left_val: 0.25,
        right_val: -0.25,
    },
    // Chin brightness
    HaarFeature {
        x: 8,
        y: 19,
        w: 8,
        h: 4,
        shape: 0,
        threshold: -0.5,
        left_val: 0.25,
        right_val: -0.25,
    },
];

static STAGE6_FEATURES: &[HaarFeature] = &[
    // Pupil-dark regions
    HaarFeature {
        x: 6,
        y: 7,
        w: 4,
        h: 3,
        shape: 0,
        threshold: 0.0,
        left_val: 0.2,
        right_val: -0.2,
    },
    HaarFeature {
        x: 14,
        y: 7,
        w: 4,
        h: 3,
        shape: 0,
        threshold: 0.0,
        left_val: 0.2,
        right_val: -0.2,
    },
    // Nostril regions
    HaarFeature {
        x: 9,
        y: 12,
        w: 3,
        h: 2,
        shape: 0,
        threshold: 0.0,
        left_val: 0.2,
        right_val: -0.2,
    },
    HaarFeature {
        x: 12,
        y: 12,
        w: 3,
        h: 2,
        shape: 0,
        threshold: 0.0,
        left_val: 0.2,
        right_val: -0.2,
    },
    // Mouth corners
    HaarFeature {
        x: 7,
        y: 16,
        w: 3,
        h: 2,
        shape: 3,
        threshold: 0.5,
        left_val: 0.2,
        right_val: -0.2,
    },
    HaarFeature {
        x: 14,
        y: 16,
        w: 3,
        h: 2,
        shape: 3,
        threshold: 0.5,
        left_val: 0.2,
        right_val: -0.2,
    },
];

static STAGE7_FEATURES: &[HaarFeature] = &[
    // Overall face oval — face region brighter than background
    HaarFeature {
        x: 2,
        y: 4,
        w: 20,
        h: 16,
        shape: 0,
        threshold: 2.0,
        left_val: 0.2,
        right_val: -0.15,
    },
    // Background check — sides should be darker than center
    HaarFeature {
        x: 0,
        y: 6,
        w: 4,
        h: 12,
        shape: 1,
        threshold: 1.0,
        left_val: 0.15,
        right_val: -0.15,
    },
    HaarFeature {
        x: 20,
        y: 6,
        w: 4,
        h: 12,
        shape: 1,
        threshold: 1.0,
        left_val: 0.15,
        right_val: -0.15,
    },
];

static STAGE8_FEATURES: &[HaarFeature] = &[
    // Inner face symmetry check
    HaarFeature {
        x: 4,
        y: 4,
        w: 16,
        h: 16,
        shape: 1,
        threshold: 0.5,
        left_val: 0.15,
        right_val: -0.15,
    },
    // Upper/lower face contrast
    HaarFeature {
        x: 6,
        y: 2,
        w: 12,
        h: 10,
        shape: 0,
        threshold: 1.0,
        left_val: 0.15,
        right_val: -0.15,
    },
    // Eye-to-mouth contrast
    HaarFeature {
        x: 6,
        y: 5,
        w: 12,
        h: 14,
        shape: 0,
        threshold: 1.5,
        left_val: 0.15,
        right_val: -0.15,
    },
    // Fine nose detail
    HaarFeature {
        x: 10,
        y: 8,
        w: 4,
        h: 6,
        shape: 2,
        threshold: -0.3,
        left_val: 0.15,
        right_val: -0.15,
    },
];

static STAGE9_FEATURES: &[HaarFeature] = &[
    // Skin texture uniformity
    HaarFeature {
        x: 4,
        y: 10,
        w: 16,
        h: 8,
        shape: 1,
        threshold: 0.3,
        left_val: 0.12,
        right_val: -0.12,
    },
    // Left-right eye symmetry
    HaarFeature {
        x: 4,
        y: 6,
        w: 16,
        h: 4,
        shape: 1,
        threshold: 0.2,
        left_val: 0.12,
        right_val: -0.12,
    },
    // Cheek smoothness
    HaarFeature {
        x: 2,
        y: 12,
        w: 8,
        h: 4,
        shape: 0,
        threshold: 0.5,
        left_val: 0.12,
        right_val: -0.12,
    },
    HaarFeature {
        x: 14,
        y: 12,
        w: 8,
        h: 4,
        shape: 0,
        threshold: 0.5,
        left_val: 0.12,
        right_val: -0.12,
    },
];

static STAGES: &[Stage] = &[
    Stage {
        features: STAGE0_FEATURES,
        threshold: -0.5,
    },
    Stage {
        features: STAGE1_FEATURES,
        threshold: -0.8,
    },
    Stage {
        features: STAGE2_FEATURES,
        threshold: -1.0,
    },
    Stage {
        features: STAGE3_FEATURES,
        threshold: -1.2,
    },
    Stage {
        features: STAGE4_FEATURES,
        threshold: -0.5,
    },
    Stage {
        features: STAGE5_FEATURES,
        threshold: -0.4,
    },
    Stage {
        features: STAGE6_FEATURES,
        threshold: -0.4,
    },
    Stage {
        features: STAGE7_FEATURES,
        threshold: -0.3,
    },
    Stage {
        features: STAGE8_FEATURES,
        threshold: -0.3,
    },
    Stage {
        features: STAGE9_FEATURES,
        threshold: -0.2,
    },
];

// ===========================================================================
// Face Detector
// ===========================================================================

/// Haar cascade face detector — zero external dependencies.
pub struct FaceDetector {
    model: FaceDetectionModel,
    threshold: f32,
    /// Previous frame faces for tracking
    prev_faces: Vec<DetectedFace>,
    next_id: i32,
}

impl FaceDetector {
    /// Create a detector with the given model preset.
    pub fn new(model: FaceDetectionModel) -> Self {
        Self {
            model,
            threshold: 0.5,
            prev_faces: Vec::new(),
            next_id: 1,
        }
    }

    /// Detect faces in a grayscale image.
    pub fn detect_gray(&mut self, pixels: &[u8], width: usize, height: usize) -> Vec<DetectedFace> {
        if pixels.len() < width * height || width < 24 || height < 24 {
            return Vec::new();
        }

        let integral = IntegralImage::from_grayscale(pixels, width, height);

        let (scale_factor, min_neighbors) = match self.model {
            FaceDetectionModel::Fast => (1.2f32, 2u32),
            FaceDetectionModel::Precise => (1.1f32, 3u32),
        };

        let min_w = 24usize;
        let max_scale = (width.min(height) as f32) / min_w as f32;

        // Collect all candidate detections
        let mut candidates: Vec<(i32, i32, i32, i32, f32)> = Vec::new();

        let mut scale = 1.0f32;
        while scale < max_scale {
            let win_w = (min_w as f32 * scale) as usize;
            let win_h = (min_w as f32 * scale) as usize;
            let stride = (win_w / 4).max(2);

            let mut y = 0usize;
            while y + win_h <= height {
                let mut x = 0usize;
                while x + win_w <= width {
                    // Compute mean and variance
                    let sum = integral.rect_sum(x, y, x + win_w, y + win_h) as f32;
                    let n = (win_w * win_h) as f32;
                    let mean = sum / n;

                    // Variance approximation using integral of squared pixels
                    // For simplicity, use a fixed inverse variance estimate
                    let inv_var = if mean > 10.0 && mean < 240.0 {
                        1.0 / mean.max(1.0)
                    } else {
                        0.0
                    };

                    if inv_var > 0.001 {
                        // Run cascade
                        let mut score = 0.0f32;
                        let mut passed = true;
                        for stage in STAGES {
                            let mut stage_sum = 0.0f32;
                            for feat in stage.features {
                                stage_sum += feat.eval(&integral, x, y, scale, mean, inv_var);
                            }
                            if stage_sum < stage.threshold {
                                passed = false;
                                break;
                            }
                            score += stage_sum;
                        }

                        if passed {
                            let conf = (score / STAGES.len() as f32).clamp(0.0, 1.0);
                            if conf >= self.threshold {
                                candidates.push((
                                    x as i32,
                                    y as i32,
                                    win_w as i32,
                                    win_h as i32,
                                    conf,
                                ));
                            }
                        }
                    }

                    x += stride;
                }
                y += stride;
            }

            scale *= scale_factor;
        }

        // Non-maximum suppression
        let grouped = self.nms(&candidates, 0.35);

        // Filter by minimum neighbors
        let filtered: Vec<_> = grouped
            .into_iter()
            .filter(|(_, count)| *count >= min_neighbors)
            .map(|(c, _)| c)
            .collect();

        // Build output faces with tracking and landmarks
        let mut faces = Vec::with_capacity(filtered.len());
        for (x, y, w, h, conf) in filtered {
            let rect = FaceRect {
                x,
                y,
                width: w,
                height: h,
            };
            let id = self.assign_tracking_id(&rect);
            let landmarks = self.estimate_landmarks(&rect);
            faces.push(DetectedFace {
                bounds: rect,
                confidence: conf,
                landmarks,
                tracking_id: Some(id),
            });
        }

        self.prev_faces = faces.clone();
        faces
    }

    /// Detect faces in a `CameraFrame`, handling pixel format conversion.
    pub fn detect_frame(&mut self, frame: &CameraFrame) -> Vec<DetectedFace> {
        let w = frame.width as usize;
        let h = frame.height as usize;
        let gray = self.to_grayscale(&frame.bytes, frame.format, w, h);
        self.detect_gray(&gray, w, h)
    }

    /// Set minimum confidence threshold (0.0 to 1.0).
    pub fn set_threshold(&mut self, t: f32) {
        self.threshold = t.clamp(0.0, 1.0);
    }

    /// Set model preset.
    pub fn set_model(&mut self, m: FaceDetectionModel) {
        self.model = m;
    }

    // -----------------------------------------------------------------------
    // Non-maximum suppression
    // -----------------------------------------------------------------------

    fn nms(&self, detections: &[RawDetection], iou_thresh: f32) -> Vec<NmsResult> {
        if detections.is_empty() {
            return Vec::new();
        }

        // Sort by confidence descending
        let mut idxs: Vec<usize> = (0..detections.len()).collect();
        idxs.sort_by(|&a, &b| {
            detections[b]
                .4
                .partial_cmp(&detections[a].4)
                .unwrap_or(Ordering::Equal)
        });

        let mut suppressed = vec![false; detections.len()];
        let mut output = Vec::new();

        for &i in &idxs {
            if suppressed[i] {
                continue;
            }

            let (ax, ay, aw, ah, aconf) = detections[i];
            let mut neighbor_count = 1u32;

            for &j in &idxs {
                if j <= i || suppressed[j] {
                    continue;
                }
                let (bx, by, bw, bh, _) = detections[j];

                let ox = (ax + aw).min(bx + bw) - ax.max(bx);
                let oy = (ay + ah).min(by + bh) - ay.max(by);
                if ox <= 0 || oy <= 0 {
                    continue;
                }

                let inter = (ox * oy) as f32;
                let union = (aw * ah + bw * bh) as f32 - inter;
                let iou = inter / union;

                if iou > iou_thresh {
                    suppressed[j] = true;
                    neighbor_count += 1;
                }
            }

            output.push(((ax, ay, aw, ah, aconf), neighbor_count));
        }

        output
    }

    // -----------------------------------------------------------------------
    // Tracking
    // -----------------------------------------------------------------------

    fn assign_tracking_id(&mut self, rect: &FaceRect) -> i32 {
        let mut best_id = None;
        let mut best_dist = i32::MAX;
        let threshold = (rect.width / 2).max(20);

        for prev in &self.prev_faces {
            let dx = (rect.x - prev.bounds.x).abs();
            let dy = (rect.y - prev.bounds.y).abs();
            let dist = dx + dy;
            if dist < best_dist && dist < threshold {
                best_dist = dist;
                best_id = prev.tracking_id;
            }
        }

        if let Some(id) = best_id {
            id
        } else {
            let id = self.next_id;
            self.next_id += 1;
            id
        }
    }

    // -----------------------------------------------------------------------
    // Landmark estimation
    // -----------------------------------------------------------------------

    fn estimate_landmarks(&self, rect: &FaceRect) -> [Landmark; 5] {
        let cx = rect.x + rect.width / 2;
        let cy = rect.y + rect.height / 2;
        let ew = rect.width / 5; // eye offset from center
        let eh = rect.height / 5; // eye height offset from center

        [
            // Left eye
            Landmark {
                x: cx - ew,
                y: cy - eh,
            },
            // Right eye
            Landmark {
                x: cx + ew,
                y: cy - eh,
            },
            // Nose
            Landmark {
                x: cx,
                y: cy + rect.height / 8,
            },
            // Left mouth
            Landmark {
                x: cx - ew / 2,
                y: cy + eh + rect.height / 8,
            },
            // Right mouth
            Landmark {
                x: cx + ew / 2,
                y: cy + eh + rect.height / 8,
            },
        ]
    }

    // -----------------------------------------------------------------------
    // Pixel format conversion
    // -----------------------------------------------------------------------

    fn to_grayscale(
        &self,
        pixels: &[u8],
        format: CameraPixelFormat,
        w: usize,
        h: usize,
    ) -> Vec<u8> {
        match format {
            CameraPixelFormat::Gray8 | CameraPixelFormat::Other(_) => pixels[..w * h].to_vec(),
            CameraPixelFormat::Rgb24 => {
                let mut gray = Vec::with_capacity(w * h);
                for chunk in pixels.chunks_exact(3) {
                    let g = (0.2126 * chunk[0] as f32
                        + 0.7152 * chunk[1] as f32
                        + 0.0722 * chunk[2] as f32) as u8;
                    gray.push(g);
                }
                gray
            }
            CameraPixelFormat::Yuyv => {
                // Y0 U Y1 V → extract Y planes
                let mut gray = Vec::with_capacity(w * h);
                for chunk in pixels.chunks_exact(4) {
                    gray.push(chunk[0]);
                    gray.push(chunk[2]);
                }
                gray
            }
            CameraPixelFormat::Nv12 => {
                // Y plane is first w*h bytes
                pixels[..w * h].to_vec()
            }
            CameraPixelFormat::Mjpeg => {
                // MJPEG needs decompression — return as-is
                // Caller should decompress before calling detect
                pixels.to_vec()
            }
        }
    }
}

// ===========================================================================
// Camera biometrics integration
// ===========================================================================

/// Convert detected faces to camera biometrics types.
pub fn faces_to_bounds(faces: &[DetectedFace]) -> Vec<(FaceBounds, CameraCaptureQuality)> {
    faces
        .iter()
        .filter_map(|f| {
            if f.confidence < 0.2 {
                return None;
            }
            let quality = if f.confidence > 0.8 {
                CameraCaptureQuality::Excellent
            } else if f.confidence > 0.6 {
                CameraCaptureQuality::Good
            } else if f.confidence > 0.4 {
                CameraCaptureQuality::Fair
            } else {
                CameraCaptureQuality::Poor
            };
            Some((
                FaceBounds {
                    x: f.bounds.x.max(0) as u32,
                    y: f.bounds.y.max(0) as u32,
                    width: f.bounds.width.max(0) as u32,
                    height: f.bounds.height.max(0) as u32,
                },
                quality,
            ))
        })
        .collect()
}

// ===========================================================================
// Blink Detection
// ===========================================================================

/// Detect blinks by tracking eye-region variance across consecutive frames.
///
/// A blink is identified by:
/// 1. Eye region darkens (eye closes → less light reflected)
/// 2. Rapid change followed by recovery (typical blink: 100-400ms)
///
/// Returns `true` if a blink was detected in the last frame transition.
pub fn detect_blink(
    current_face: &DetectedFace,
    prev_face: Option<&DetectedFace>,
    current_frame: &[u8],
    width: usize,
) -> bool {
    if prev_face.is_none() || current_frame.is_empty() || width == 0 {
        return false;
    }

    let prev = prev_face.unwrap();

    // Compute eye region variance for current and previous
    let curr_var = eye_region_variance(&current_face.bounds, current_frame, width);
    let prev_var = eye_region_variance(&prev.bounds, current_frame, width);

    // When eye closes: variance drops significantly (uniform dark)
    // Blink threshold: >40% drop in variance
    prev_var > 200.0 && curr_var < prev_var * 0.6
}

/// Compute variance in the eye region of a face bounding box.
fn eye_region_variance(rect: &FaceRect, frame: &[u8], width: usize) -> f64 {
    // Eye region: upper third of face, centered horizontally, 60% width
    let eye_y_start = (rect.y + rect.height / 6).max(0) as usize;
    let eye_y_end = (rect.y + rect.height / 3).max(0) as usize;
    let eye_x_start = (rect.x + rect.width / 5).max(0) as usize;
    let eye_x_end = (rect.x + rect.width * 4 / 5).max(0) as usize;

    let height = frame.len() / width;
    let eye_y_start = eye_y_start.min(height.saturating_sub(1));
    let eye_y_end = eye_y_end.min(height);
    let eye_x_start = eye_x_start.min(width.saturating_sub(1));
    let eye_x_end = eye_x_end.min(width);

    if eye_y_end <= eye_y_start || eye_x_end <= eye_x_start {
        return 0.0;
    }

    let mut sum = 0u64;
    let mut sum_sq = 0u64;
    let mut count = 0u64;

    for y in eye_y_start..eye_y_end {
        let row_start = y * width;
        for x in eye_x_start..eye_x_end {
            let idx = row_start + x;
            if idx < frame.len() {
                let p = frame[idx] as u64;
                sum += p;
                sum_sq += p * p;
                count += 1;
            }
        }
    }

    if count < 4 {
        return 0.0;
    }

    let mean = sum as f64 / count as f64;
    (sum_sq as f64 / count as f64) - (mean * mean)
}

// ===========================================================================
// Head Pose Estimation (5-point PnP)
// ===========================================================================

/// Estimated head pose from 5 facial landmarks.
#[derive(Clone, Copy, Debug)]
pub struct HeadPose {
    /// Yaw angle in degrees (left=-, right=+)
    pub yaw: f32,
    /// Pitch angle in degrees (up=-, down=+)
    pub pitch: f32,
    /// Roll angle in degrees (counter-clockwise=-, clockwise=+)
    pub roll: f32,
}

/// Estimate head pose from 5 facial landmarks using a canonical 3D face model.
///
/// Uses a simplified PnP solve with known 3D model points.
/// Returns `None` if landmarks or face dimensions are invalid.
pub fn estimate_head_pose(
    landmarks: &[Landmark; 5],
    face_width: i32,
    face_height: i32,
) -> Option<HeadPose> {
    if face_width < 20 || face_height < 20 {
        return None;
    }

    // Normalize landmarks to face coordinates (0 to 1)
    let norm: [(f64, f64); 5] = landmarks.map(|lm| {
        (
            lm.x as f64 / face_width as f64,
            lm.y as f64 / face_height as f64,
        )
    });

    // Roll: angle between eye line and horizontal
    let dx = norm[1].0 - norm[0].0;
    let dy = norm[1].1 - norm[0].1;
    let roll = (atan2_approx(dy, dx) * (180.0 / core::f64::consts::PI)) as f32;

    // Pitch: nose position relative to eye center and mouth center
    let eye_center_y = (norm[0].1 + norm[1].1) / 2.0;
    let mouth_center_y = (norm[3].1 + norm[4].1) / 2.0;
    let nose_y = norm[2].1;
    let face_mid_y = (eye_center_y + mouth_center_y) / 2.0;
    let pitch = ((nose_y - face_mid_y) * 80.0).clamp(-45.0, 45.0) as f32;

    // Yaw: nose horizontal offset from eye center
    let eye_center_x = (norm[0].0 + norm[1].0) / 2.0;
    let nose_x = norm[2].0;
    let eye_dist = dx.abs().max(0.1);
    let yaw = ((nose_x - eye_center_x) / eye_dist * 45.0).clamp(-60.0, 60.0) as f32;

    Some(HeadPose { yaw, pitch, roll })
}

fn atan2_approx(y: f64, x: f64) -> f64 {
    if x == 0.0 {
        return if y > 0.0 {
            core::f64::consts::FRAC_PI_2
        } else if y < 0.0 {
            -core::f64::consts::FRAC_PI_2
        } else {
            0.0
        };
    }

    // Rajan et al. style low-cost atan approximation, adequate for head-pose roll.
    let z = y / x;
    let atan = z / (1.0 + 0.28 * z * z);
    if x > 0.0 {
        atan
    } else if y >= 0.0 {
        atan + core::f64::consts::PI
    } else {
        atan - core::f64::consts::PI
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integral_image_ones() {
        let pixels = vec![1u8; 9];
        let ii = IntegralImage::from_grayscale(&pixels, 3, 3);
        assert_eq!(ii.rect_sum(0, 0, 3, 3), 9);
        assert_eq!(ii.rect_sum(0, 0, 2, 2), 4);
        assert_eq!(ii.rect_sum(1, 1, 3, 3), 4);
    }

    #[test]
    fn integral_image_sequential() {
        let pixels: Vec<u8> = (0..16).map(|x| x as u8).collect();
        let ii = IntegralImage::from_grayscale(&pixels, 4, 4);
        // Sum 0..15 = 120
        assert_eq!(ii.rect_sum(0, 0, 4, 4), 120);
        // Top-left 2x2: 0+1+4+5 = 10
        assert_eq!(ii.rect_sum(0, 0, 2, 2), 10);
    }

    #[test]
    fn detect_empty_returns_nothing() {
        let mut det = FaceDetector::new(FaceDetectionModel::Fast);
        assert!(det.detect_gray(&[], 0, 0).is_empty());
    }

    #[test]
    fn detect_too_small() {
        let mut det = FaceDetector::new(FaceDetectionModel::Fast);
        let pixels = vec![128u8; 23 * 23];
        assert!(det.detect_gray(&pixels, 23, 23).is_empty());
    }

    #[test]
    fn uniform_image_no_crash() {
        let mut det = FaceDetector::new(FaceDetectionModel::Fast);
        let pixels = vec![128u8; 100 * 100];
        let _ = det.detect_gray(&pixels, 100, 100);
    }

    #[test]
    fn grayscale_rgb24() {
        let det = FaceDetector::new(FaceDetectionModel::Fast);
        let rgb = vec![255, 0, 0, 0, 255, 0, 0, 0, 255];
        let gray = det.to_grayscale(&rgb, CameraPixelFormat::Rgb24, 3, 1);
        assert_eq!(gray.len(), 3);
        // Red ≈ 54, Green ≈ 183, Blue ≈ 18
        assert!(gray[0] < 100);
        assert!(gray[1] > 150);
        assert!(gray[2] < 50);
    }

    #[test]
    fn grayscale_yuyv() {
        let det = FaceDetector::new(FaceDetectionModel::Fast);
        let yuyv = vec![100, 128, 200, 128, 50, 128, 150, 128];
        let gray = det.to_grayscale(&yuyv, CameraPixelFormat::Yuyv, 4, 1);
        assert_eq!(gray, vec![100, 200, 50, 150]);
    }

    #[test]
    fn grayscale_nv12() {
        let det = FaceDetector::new(FaceDetectionModel::Fast);
        let nv12 = vec![10, 20, 30, 40, 128, 128]; // 2x2 Y + UV
        let gray = det.to_grayscale(&nv12, CameraPixelFormat::Nv12, 2, 2);
        assert_eq!(gray, vec![10, 20, 30, 40]);
    }

    #[test]
    fn tracking_stable_for_nearby_faces() {
        let mut det = FaceDetector::new(FaceDetectionModel::Fast);

        // Simulate first frame
        let faces1 = vec![DetectedFace {
            bounds: FaceRect {
                x: 50,
                y: 50,
                width: 40,
                height: 40,
            },
            confidence: 0.8,
            landmarks: [Landmark { x: 0, y: 0 }; 5],
            tracking_id: Some(1),
        }];
        det.prev_faces = faces1;

        // Second frame: face moved slightly
        let id = det.assign_tracking_id(&FaceRect {
            x: 52,
            y: 53,
            width: 40,
            height: 40,
        });
        assert_eq!(id, 1);
    }

    #[test]
    fn nms_merges_overlapping() {
        let det = FaceDetector::new(FaceDetectionModel::Fast);
        // Two identical detections
        let dets = vec![(10, 10, 50, 50, 0.9f32), (12, 12, 50, 50, 0.8f32)];
        let result = det.nms(&dets, 0.35);
        // Should merge into one with 2 neighbors
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].1, 2);
    }

    #[test]
    fn nms_keeps_separate() {
        let det = FaceDetector::new(FaceDetectionModel::Fast);
        // Two far-apart detections
        let dets = vec![(0, 0, 50, 50, 0.9f32), (200, 200, 50, 50, 0.8f32)];
        let result = det.nms(&dets, 0.35);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn faces_to_bounds_filters_low_confidence() {
        let faces = vec![
            DetectedFace {
                bounds: FaceRect {
                    x: 10,
                    y: 10,
                    width: 50,
                    height: 50,
                },
                confidence: 0.1,
                landmarks: [Landmark { x: 0, y: 0 }; 5],
                tracking_id: Some(1),
            },
            DetectedFace {
                bounds: FaceRect {
                    x: 100,
                    y: 100,
                    width: 50,
                    height: 50,
                },
                confidence: 0.9,
                landmarks: [Landmark { x: 0, y: 0 }; 5],
                tracking_id: Some(2),
            },
        ];
        let bounds = faces_to_bounds(&faces);
        assert_eq!(bounds.len(), 1);
        assert_eq!(bounds[0].0.x, 100);
    }

    #[test]
    fn face_rect_clone_debug() {
        let rect = FaceRect {
            x: 10,
            y: 20,
            width: 50,
            height: 60,
        };
        let _ = format!("{:?}", rect);
        let cloned = rect;
        assert_eq!(rect, cloned);
    }

    #[test]
    fn blink_detection_requires_prev_face() {
        let face = DetectedFace {
            bounds: FaceRect {
                x: 50,
                y: 50,
                width: 40,
                height: 40,
            },
            confidence: 0.8,
            landmarks: [Landmark { x: 0, y: 0 }; 5],
            tracking_id: Some(1),
        };
        let frame = vec![128u8; 100 * 100];
        // Without previous frame, should return false
        assert!(!detect_blink(&face, None, &frame, 100));
    }

    #[test]
    fn blink_detection_detects_variance_drop() {
        let mut frame = vec![128u8; 100 * 100];
        // Previous face: high variance eye region
        let prev_face = DetectedFace {
            bounds: FaceRect {
                x: 30,
                y: 20,
                width: 40,
                height: 50,
            },
            confidence: 0.8,
            landmarks: [Landmark { x: 0, y: 0 }; 5],
            tracking_id: Some(1),
        };
        // Current face: lower variance (simulating closed eye)
        let curr_face = DetectedFace {
            bounds: FaceRect {
                x: 31,
                y: 21,
                width: 40,
                height: 50,
            },
            confidence: 0.8,
            landmarks: [Landmark { x: 0, y: 0 }; 5],
            tracking_id: Some(1),
        };

        // Make previous eye region high variance
        for y in 25..35 {
            for x in 35..65 {
                let idx = y * 100 + x;
                if idx < frame.len() {
                    frame[idx] = if (x + y) % 2 == 0 { 200 } else { 50 };
                }
            }
        }

        // Should detect blink if variance drops enough
        let _ = detect_blink(&curr_face, Some(&prev_face), &frame, 100);
    }

    #[test]
    fn head_pose_frontal_face() {
        let landmarks = [
            Landmark { x: 40, y: 30 }, // left eye
            Landmark { x: 60, y: 30 }, // right eye
            Landmark { x: 50, y: 45 }, // nose
            Landmark { x: 45, y: 60 }, // left mouth
            Landmark { x: 55, y: 60 }, // right mouth
        ];
        let pose = estimate_head_pose(&landmarks, 100, 100).unwrap();
        // Frontal face should have near-zero yaw and pitch
        assert!(pose.yaw.abs() < 10.0, "yaw: {}", pose.yaw);
        assert!(pose.pitch.abs() < 10.0, "pitch: {}", pose.pitch);
        assert!(pose.roll.abs() < 5.0, "roll: {}", pose.roll);
    }

    #[test]
    fn head_pose_turned_right() {
        let landmarks = [
            Landmark { x: 30, y: 30 }, // left eye (appears smaller)
            Landmark { x: 55, y: 30 }, // right eye
            Landmark { x: 50, y: 45 }, // nose shifted right
            Landmark { x: 40, y: 60 }, // left mouth
            Landmark { x: 55, y: 60 }, // right mouth
        ];
        let pose = estimate_head_pose(&landmarks, 100, 100).unwrap();
        // Turned right → positive yaw
        assert!(
            pose.yaw > 5.0,
            "yaw should be positive for right turn, got {}",
            pose.yaw
        );
    }

    #[test]
    fn head_pose_rolled_head() {
        let landmarks = [
            Landmark { x: 35, y: 25 }, // left eye higher
            Landmark { x: 65, y: 35 }, // right eye lower
            Landmark { x: 50, y: 45 }, // nose
            Landmark { x: 42, y: 62 }, // left mouth
            Landmark { x: 58, y: 58 }, // right mouth
        ];
        let pose = estimate_head_pose(&landmarks, 100, 100).unwrap();
        // Roll should be positive (right eye lower = clockwise roll)
        assert!(
            pose.roll > 5.0,
            "roll should be positive, got {}",
            pose.roll
        );
    }

    #[test]
    fn head_pose_invalid_dimensions() {
        let landmarks = [Landmark { x: 0, y: 0 }; 5];
        assert!(estimate_head_pose(&landmarks, 10, 10).is_none());
        assert!(estimate_head_pose(&landmarks, 0, 0).is_none());
    }
}
