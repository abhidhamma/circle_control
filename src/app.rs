use egui::{Color32, Pos2, Slider, Vec2, vec2};
// C++의 glm 라이브러리 대신 glam 크레이트를 사용합니다.
use glam::{Vec3, vec3};

// C++의 Ray, Hit, Sphere, Raytracer 클래스에 해당하는 Rust 구조체들을 정의합니다.

/// 광선의 시작점으로부터 충돌 지점까지의 정보를 담습니다.
struct Hit {
    /// 광선의 시작점부터 충돌 지점까지의 거리. 음수이면 충돌하지 않음을 의미합니다.
    distance: f32,
    /// 광선과 구가 충돌한 지점의 3D 월드 좌표.
    point: Vec3,
    /// 충돌 지점에서 구 표면의 법선 벡터(normal vector).
    normal: Vec3,
}

/// 3D 공간 상의 광선을 나타냅니다.
struct Ray {
    /// 광선의 시작점 (origin).
    origin: Vec3,
    /// 광선의 방향 벡터 (direction). 단위 벡터여야 합니다.
    direction: Vec3,
}

/// 3D 공간 상의 구를 나타냅니다.
struct Sphere {
    center: Vec3,
    radius: f32,
    color: Vec3, // f32 기반 벡터로 색상을 다루면 계산이 편리합니다.
}

impl Sphere {
    /// 주어진 광선(ray)과 구의 충돌을 계산합니다.
    ///
    /// # Arguments
    /// * `ray` - 충돌을 검사할 광선.
    ///
    /// # Returns
    /// * `Hit` - 충돌 정보를 담은 구조체. 충돌하지 않으면 `distance`가 음수인 `Hit`이 반환됩니다.
    ///
    /// # 수학적 원리
    /// 광선의 방정식: P(t) = origin + t * direction
    /// 구의 방정식: ||P - center||² = radius²
    ///
    /// 위 두 식을 결합하여 t에 대한 2차 방정식을 만듭니다: At² + Bt + C = 0
    /// A = direction · direction (direction이 단위 벡터이므로 1)
    /// B = 2 * direction · (origin - center)
    /// C = (origin - center) · (origin - center) - radius²
    ///
    /// 판별식(nabla): (B/2)² - C >= 0 이면 실근(충돌점)이 존재합니다.
    fn intersect_ray_collision(&self, ray: &Ray) -> Hit {
        // origin - center 벡터. 자주 사용되므로 변수에 저장합니다.
        let oc = ray.origin - self.center;

        // a = ray.direction.dot(ray.direction) 이지만, direction이 단위벡터이므로 a는 1.0 입니다.
        // 따라서 2차 방정식은 t² + 2 * (d·oc)t + oc² - r² = 0 이 됩니다.
        // b = 2 * (d·oc) 이고, c = oc² - r² 입니다.
        // 근의 공식에서 b 대신 b/2 (half_b)를 사용하면 계산이 간결해집니다.
        let half_b = ray.direction.dot(oc);
        let c = oc.length_squared() - self.radius * self.radius;

        // 판별식 (nabla) = (b/2)² - ac. 여기서 a=1 이므로 (b/2)² - c 입니다.
        let discriminant = half_b * half_b - c;

        // 판별식이 0보다 작으면 광선이 구와 만나지 않습니다.
        if discriminant < 0.0 {
            return Hit {
                distance: -1.0,
                point: Vec3::ZERO,
                normal: Vec3::ZERO,
            };
        }

        // 두 개의 실근(충돌 거리)을 계산합니다.
        // 광선이 구를 뚫고 들어가는 지점과 나가는 지점에 해당합니다.
        let sqrt_discriminant = discriminant.sqrt();
        let t1 = -half_b - sqrt_discriminant;
        let t2 = -half_b + sqrt_discriminant;

        // 두 근 중 더 작은 양수 값이 카메라에 더 가까운 충돌점입니다.
        let distance = if t1 >= 0.0 {
            t1
        } else if t2 >= 0.0 {
            t2
        } else {
            // 두 근 모두 음수이면, 충돌 지점이 광선의 시작점 뒤에 있다는 의미이므로 충돌하지 않은 것으로 처리합니다.
            -1.0
        };

        if distance < 0.0 {
            return Hit {
                distance: -1.0,
                point: Vec3::ZERO,
                normal: Vec3::ZERO,
            };
        }

        // 충돌 정보를 계산하여 Hit 구조체를 채웁니다.
        let point = ray.origin + ray.direction * distance;
        let normal = (point - self.center).normalize();
        Hit {
            distance,
            point,
            normal,
        }
    }
}

/// 레이트레이싱 계산을 담당합니다.
struct Raytracer {
    width: i32,
    height: i32,
    sphere: Sphere,
}

impl Raytracer {
    fn new(width: i32, height: i32) -> Self {
        Self {
            width,
            height,
            sphere: Sphere {
                center: vec3(0.0, 0.0, 0.5),
                radius: 0.4,
                color: vec3(1.0, 1.0, 1.0), // C++ 예제와 동일하게 흰색으로 시작
            },
        }
    }

    /// 2D 스크린 좌표를 3D 월드 좌표로 변환합니다.
    fn transform_screen_to_world(&self, pos_screen: Vec2) -> Vec3 {
        let x_scale = 2.0 / (self.width - 1) as f32;
        let y_scale = 2.0 / (self.height - 1) as f32;
        let aspect = self.width as f32 / self.height as f32;

        // 3차원 공간으로 확장 (z좌표는 0.0으로 가정)
        vec3(
            (pos_screen.x * x_scale - 1.0) * aspect,
            -pos_screen.y * y_scale + 1.0,
            0.0,
        )
    }

    /// 주어진 광선을 추적하여 최종 색상을 계산합니다.
    fn trace_ray(&self, ray: &Ray) -> Vec3 {
        let hit = self.sphere.intersect_ray_collision(ray);

        if hit.distance < 0.0 {
            // 충돌하지 않으면 검은색 반환
            vec3(0.0, 0.0, 0.0)
        } else {
            // C++ 예제처럼 깊이(distance)를 곱해 입체감을 표현합니다.
            self.sphere.color * hit.distance
        }
    }

    /// 모든 픽셀에 대해 레이트레이싱을 수행하여 픽셀 버퍼를 채웁니다.
    fn render(&self, pixels: &mut [Color32]) {
        // rayon을 사용해 병렬 처리 (C++의 #pragma omp parallel for와 유사)
        use rayon::prelude::*;

        pixels.par_iter_mut().enumerate().for_each(|(idx, pixel)| {
            let i = idx % self.width as usize;
            let j = idx / self.width as usize;

            let pos_world = self
                .transform_screen_to_world(vec2(i as f32, j as f32));

            // Orthographic projection (정투영): 모든 광선이 z축 방향으로 평행하게 나아갑니다.
            let ray_dir = vec3(0.0, 0.0, 1.0);
            let pixel_ray = Ray {
                origin: pos_world,
                direction: ray_dir,
            };

            // 광선을 추적하여 색상을 계산합니다.
            let color_vec = self.trace_ray(&pixel_ray);

            // 계산된 색상(Vec3, 0.0~1.0)을 Color32(u8, 0~255)로 변환합니다.
            *pixel = Color32::from_rgb(
                (color_vec.x.clamp(0.0, 1.0) * 255.0) as u8,
                (color_vec.y.clamp(0.0, 1.0) * 255.0) as u8,
                (color_vec.z.clamp(0.0, 1.0) * 255.0) as u8,
            );
        });
    }
}

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct TemplateApp {
    // serde(skip)은 이 필드를 저장/로드 상태에서 제외시킵니다.
    #[serde(skip)]
    raytracer: Raytracer,

    // UI 컨트롤과 직접 연결될 변수들
    center_x: f32,
    center_y: f32,
    center_z: f32,
    radius: f32,
    // egui의 Color32는 [u8; 4] 이므로, f32 기반 색상과 변환이 필요합니다.
    color: [f32; 3],
}

mod consts {
    use egui::Color32;
    pub const MAX_RESOLUTION_X: i32 = 1280;
    pub const MAX_RESOLUTION_Y: i32 = 720;
    pub const DEFAULT_BACKGROUND_COLOR: Color32 =
        Color32::from_gray(30);
}

impl Default for TemplateApp {
    fn default() -> Self {
        let raytracer = Raytracer::new(
            consts::MAX_RESOLUTION_X,
            consts::MAX_RESOLUTION_Y,
        );
        Self {
            center_x: raytracer.sphere.center.x,
            center_y: raytracer.sphere.center.y,
            center_z: raytracer.sphere.center.z,
            radius: raytracer.sphere.radius,
            color: [
                raytracer.sphere.color.x,
                raytracer.sphere.color.y,
                raytracer.sphere.color.z,
            ],
            raytracer,
        }
    }
}

impl TemplateApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        if let Some(storage) = cc.storage {
            return eframe::get_value(storage, eframe::APP_KEY)
                .unwrap_or_default();
        }
        Self::default()
    }
}

impl eframe::App for TemplateApp {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    fn update(
        &mut self,
        ctx: &egui::Context,
        _frame: &mut eframe::Frame,
    ) {
        // UI 컨트롤러 값들을 실제 Raytracer의 Sphere 데이터에 반영합니다.
        self.raytracer.sphere.center.x = self.center_x;
        self.raytracer.sphere.center.y = self.center_y;
        self.raytracer.sphere.center.z = self.center_z;
        self.raytracer.sphere.radius = self.radius;
        self.raytracer.sphere.color = Vec3::from_slice(&self.color);

        egui::SidePanel::left("control_panel").show(ctx, |ui| {
            ui.heading("Sphere Controls");
            ui.add(
                Slider::new(&mut self.center_x, -1.0..=1.0)
                    .text("Center X"),
            );
            ui.add(
                Slider::new(&mut self.center_y, -1.0..=1.0)
                    .text("Center Y"),
            );
            ui.add(
                Slider::new(&mut self.center_z, -1.0..=1.0)
                    .text("Center Z"),
            );
            ui.add(
                Slider::new(&mut self.radius, 0.0..=1.0)
                    .text("Radius"),
            );
            ui.label("Color:");
            // color_edit_button_rgb는 [u8; 3]을 받으므로, f32 슬라이더로 대체합니다.
            ui.color_edit_button_rgb(&mut self.color);
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            let width = self.raytracer.width as usize;
            let height = self.raytracer.height as usize;

            // 1. 픽셀 데이터를 담을 버퍼를 생성합니다.
            let mut pixels: Vec<Color32> = vec![
                    consts::DEFAULT_BACKGROUND_COLOR;
                    width * height
                ];

            // 2. Raytracer가 픽셀 버퍼를 채웁니다. (CPU 렌더링)
            self.raytracer.render(&mut pixels);

            // 3. 픽셀 데이터로 egui::ColorImage를 생성합니다.
            let image = egui::ColorImage {
                size: [width, height],
                source_size: vec2(width as f32, height as f32), // 이 줄을 추가합니다.
                pixels,
            };

            // 4. ColorImage를 GPU 텍스처로 로드하고 화면에 그립니다.
            let texture = ctx.load_texture(
                "sphere_canvas",
                image,
                egui::TextureOptions::NEAREST,
            );
            ui.image((texture.id(), texture.size_vec2()));
        });

        // UI가 변경되면 지속적으로 화면을 다시 그리도록 요청합니다.
        ctx.request_repaint();
    }
}
