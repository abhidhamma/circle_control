use egui::{Color32, Slider, Vec2, vec2};
// C++의 glm 대신 glam 크레이트 사용
use glam::{Vec3, vec3};

// Ray, Hit, Sphere, Raytracer Rust 구조체 정의

/// 광선 충돌 정보
struct Hit {
    /// 광선 시작점부터 충돌 지점까지의 거리 (음수: 충돌 없음)
    distance: f32,
    /// 충돌 지점 (3D 월드 좌표)
    point: Vec3,
    /// 충돌 지점의 법선 벡터 (normal)
    normal: Vec3,
}

/// 3D 공간의 광선
struct Ray {
    /// 광선 시작점
    origin: Vec3,
    /// 광선 방향 벡터(단위 벡터)
    direction: Vec3,
}

/// 3D 공간의 구
struct Sphere {
    center: Vec3,
    radius: f32,
    // 퐁 리플렉션 모델을 위한 재질(material) 속성
    /// 주변광(Ambient) 색상
    amb: Vec3,
    /// 확산광(Diffuse) 색상
    diff: Vec3,
    /// 반사광(Specular) 색상
    spec: Vec3,
    /// 반사광 계수 (specular coefficient)
    ks: f32,
    /// 반사광 지수 (shininess)
    alpha: f32,
}

impl Sphere {
    /// 광선과 구의 충돌 계산
    ///
    /// # 매개변수
    /// ray - 충돌 검사할 광선
    ///
    /// # 리턴
    /// Hit - 충돌 정보. 미충돌 시 distance는 음수
    ///
    /// # 공식
    /// 직선의 방정식: x = 시작점 + 거리 * 방향
    /// 구의 방정식: ||x - 원의중심||² = 반지름²
    ///
    /// 두 식을 결합해 x에 대한 2차 방정식을 만듦: Ax² + Bx + C = 0
    /// 판별식: (B/2)² - C >= 0 이면 실근(충돌점) 존재
    fn intersect_ray_collision(&self, ray: &Ray) -> Hit {
        let oc = ray.origin - self.center;
        // C++ 코드의 2차 방정식 근의 공식과 동일한 로직으로 수정
        // a는 ray.direction이 단위 벡터이므로 1.0
        let b = 2.0 * ray.direction.dot(oc);
        let c = oc.length_squared() - self.radius * self.radius;

        // 판별식
        let discriminant = b * b - 4.0 * c;

        // 판별식이 0보다 작으면 실근 없음 (충돌 안 함)
        if discriminant < 0.0 {
            return Hit {
                distance: -1.0,
                point: Vec3::ZERO,
                normal: Vec3::ZERO,
            };
        }

        // 충돌하는 경우 두 실근(충돌 거리) 계산
        let sqrt_discriminant = discriminant.sqrt();
        let t1 = (-b - sqrt_discriminant) / 2.0;
        let t2 = (-b + sqrt_discriminant) / 2.0;

        // 두 근 중 더 작은 양수 값이 카메라에 가까운 충돌점
        let distance = if t1 >= 0.0 && (t1 < t2 || t2 < 0.0) {
            t1
        } else if t2 >= 0.0 {
            t2
        } else {
            // 두 근 모두 음수이면 광선위 시작점 뒤쪽이므로 충돌 아님
            -1.0
        };

        if distance < 0.0 {
            return Hit {
                distance: -1.0,
                point: Vec3::ZERO,
                normal: Vec3::ZERO,
            };
        }

        // 충돌 정보 계산 및 반환
        let point = ray.origin + ray.direction * distance;
        let normal = (point - self.center).normalize();
        Hit {
            distance,
            point,
            normal,
        }
    }
}

/// 점 광원
struct Light {
    pos: Vec3,
}

/// 레이트레이싱 계산 담당
struct Raytracer {
    width: i32,
    height: i32,
    sphere: Sphere,
    light: Light,
}

impl Raytracer {
    fn new(width: i32, height: i32) -> Self {
        Self {
            width,
            height,
            sphere: Sphere {
                center: vec3(0.0, 0.0, 0.5),
                radius: 0.4,
                amb: vec3(0.0, 0.0, 0.0),
                diff: vec3(0.0, 0.0, 1.0),
                spec: vec3(1.0, 1.0, 1.0),
                ks: 0.8,
                alpha: 9.0,
            },
            light: Light {
                pos: vec3(0.0, 0.0, -1.0),
            },
        }
    }

    /// 2D 스크린 좌표를 3D 월드 좌표로 변환
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

    /// 주어진 광선을 추적해 최종 색상 계산
    fn trace_ray(&self, ray: &Ray) -> Vec3 {
        let hit = self.sphere.intersect_ray_collision(ray);

        if hit.distance < 0.0 {
            // 충돌하지 않으면 검은색 반환
            vec3(0.0, 0.0, 0.0)
        } else {
            // 퐁 리플렉션 모델로 조명 계산
            // 1. Diffuse (확산광)
            let dir_to_light =
                (self.light.pos - hit.point).normalize();
            let diff = hit.normal.dot(dir_to_light).max(0.0);

            // 2. Specular (반사광)
            let reflect_dir =
                2.0 * hit.normal.dot(dir_to_light) * hit.normal
                    - dir_to_light;
            let specular = (-ray.direction)
                .dot(reflect_dir)
                .max(0.0)
                .powf(self.sphere.alpha);

            // 3. Ambient + Diffuse + Specular
            self.sphere.amb
                + self.sphere.diff * diff
                + self.sphere.spec * specular * self.sphere.ks
        }
    }

    /// 모든 픽셀에 대해 레이트레이싱을 수행해 픽셀 버퍼를 채움
    fn render(&self, pixels: &mut [Color32]) {
        // rayon을 사용해 병렬 처리 (C++의 #pragma omp parallel for와 유사)
        use rayon::prelude::*;

        pixels.par_iter_mut().enumerate().for_each(|(idx, pixel)| {
            let i = idx % self.width as usize;
            let j = idx / self.width as usize;

            let pos_world = self
                .transform_screen_to_world(vec2(i as f32, j as f32));

            // Orthographic projection (정투영): 모든 광선이 z축으로 평행하게 나아감
            let ray_dir = vec3(0.0, 0.0, 1.0);
            let pixel_ray = Ray {
                origin: pos_world,
                direction: ray_dir,
            };

            // 광선을 추적해 색상 계산
            let color_vec = self.trace_ray(&pixel_ray);

            // 계산된 색상(Vec3, 0.0~1.0)을 Color32(u8, 0~255)로 변환
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
    // serde(skip): 저장/로드 상태에서 제외
    #[serde(skip)]
    raytracer: Raytracer,

    // UI 컨트롤과 직접 연결될 변수들
    center_x: f32,
    center_y: f32,
    center_z: f32,
    radius: f32,
    light_x: f32,
    light_y: f32,
    light_z: f32,
    amb_color: [f32; 3],
    diff_color: [f32; 3],
    spec_color: [f32; 3],
    spec_coeff: f32,
    spec_power: f32,
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
            light_x: raytracer.light.pos.x,
            light_y: raytracer.light.pos.y,
            light_z: raytracer.light.pos.z,
            amb_color: raytracer.sphere.amb.to_array(),
            diff_color: raytracer.sphere.diff.to_array(),
            spec_color: raytracer.sphere.spec.to_array(),
            spec_coeff: raytracer.sphere.ks,
            spec_power: raytracer.sphere.alpha,
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
        // UI 컨트롤러 값을 Raytracer 데이터에 반영
        self.raytracer.sphere.center.x = self.center_x;
        self.raytracer.sphere.center.y = self.center_y;
        self.raytracer.sphere.center.z = self.center_z;
        self.raytracer.sphere.radius = self.radius;
        self.raytracer.light.pos.x = self.light_x;
        self.raytracer.light.pos.y = self.light_y;
        self.raytracer.light.pos.z = self.light_z;
        self.raytracer.sphere.amb = Vec3::from_slice(&self.amb_color);
        self.raytracer.sphere.diff =
            Vec3::from_slice(&self.diff_color);
        self.raytracer.sphere.spec =
            Vec3::from_slice(&self.spec_color);
        self.raytracer.sphere.ks = self.spec_coeff;
        self.raytracer.sphere.alpha = self.spec_power;

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

            ui.separator();
            ui.heading("Light Controls");
            ui.add(
                Slider::new(&mut self.light_x, -2.0..=2.0)
                    .text("Light X"),
            );
            ui.add(
                Slider::new(&mut self.light_y, -2.0..=2.0)
                    .text("Light Y"),
            );
            ui.add(
                Slider::new(&mut self.light_z, -2.0..=2.0)
                    .text("Light Z"),
            );

            ui.separator();
            ui.heading("Material Controls");
            ui.label("Ambient Color:");
            ui.color_edit_button_rgb(&mut self.amb_color);
            ui.label("Diffuse Color:");
            ui.color_edit_button_rgb(&mut self.diff_color);
            ui.label("Specular Color:");
            ui.color_edit_button_rgb(&mut self.spec_color);
            ui.add(
                Slider::new(&mut self.spec_power, 0.0..=100.0)
                    .text("Specular Power"),
            );
            ui.add(
                Slider::new(&mut self.spec_coeff, 0.0..=1.0)
                    .text("Specular Coeff"),
            );
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            let width = self.raytracer.width as usize;
            let height = self.raytracer.height as usize;

            // 1. 픽셀 데이터를 담을 버퍼 생성
            let mut pixels: Vec<Color32> = vec![
                    consts::DEFAULT_BACKGROUND_COLOR;
                    width * height
                ];

            // 2. Raytracer가 픽셀 버퍼를 채움 (CPU 렌더링)
            self.raytracer.render(&mut pixels);

            // 3. 픽셀 데이터로 이미지 생성
            let image = egui::ColorImage::from_rgba_unmultiplied(
                [width, height],
                bytemuck::cast_slice(&pixels),
            );

            // 4. ColorImage를 GPU 텍스처로 로드하고 화면에 그림
            let texture = ctx.load_texture(
                "sphere_canvas",
                image,
                egui::TextureOptions::NEAREST,
            );
            ui.image((texture.id(), texture.size_vec2()));
        });

        // UI 변경 시 화면을 다시 그리도록 요청
        ctx.request_repaint();
    }
}
