// h:\coding\rustWorkspace\graphics\circle_control\src\app.rs

use egui::{Color32, Slider, Vec2, vec2};
use glam::{Vec3, vec3};
use std::sync::Arc;

// C++의 Object, Sphere, Light, Hit, Ray 클래스/구조체를 Rust의 struct로 번역합니다.
// Rust에서는 상속 대신 Trait을 사용해 다형성을 구현합니다.

// C++: class Object (가상함수 포함)
// Rust: trait Object (동적 디스패치를 위해 Box<dyn Object> 또는 Arc<dyn Object>와 함께 사용)
trait Object: Send + Sync {
    fn check_ray_collision(&self, ray: &Ray) -> Hit;
    fn ambient(&self) -> Vec3;
    fn diffuse(&self) -> Vec3;
    fn specular(&self) -> Vec3;
    fn alpha(&self) -> f32;
}

// C++: class Hit
struct Hit {
    distance: f32,
    point: Vec3,
    normal: Vec3,
    // C++: std::shared_ptr<Object> obj;
    // Rust: 충돌한 객체의 재질 정보에 접근하기 위해 Arc<dyn Object>를 사용합니다.
    object: Option<Arc<dyn Object>>,
}

// C++: class Ray
struct Ray {
    origin: Vec3,
    direction: Vec3,
}

// C++: class Sphere : public Object
struct Sphere {
    center: Vec3,
    radius: f32,
    amb: Vec3,
    diff: Vec3,
    spec: Vec3,
    alpha: f32,
}

// Sphere에 Object 트레이트를 구현합니다.
impl Object for Sphere {
    fn check_ray_collision(&self, ray: &Ray) -> Hit {
        let oc = ray.origin - self.center;
        let b = 2.0 * ray.direction.dot(oc);
        let c = oc.length_squared() - self.radius * self.radius;
        let discriminant = b * b - 4.0 * c;

        if discriminant < 0.0 {
            return Hit {
                distance: -1.0,
                point: Vec3::ZERO,
                normal: Vec3::ZERO,
                object: None,
            };
        }

        let sqrt_discriminant = discriminant.sqrt();
        let t1 = (-b - sqrt_discriminant) / 2.0;
        let t2 = (-b + sqrt_discriminant) / 2.0;

        let distance = if t1 >= 0.0 && (t1 < t2 || t2 < 0.0) {
            t1
        } else if t2 >= 0.0 {
            t2
        } else {
            -1.0
        };

        if distance < 0.0 {
            return Hit {
                distance: -1.0,
                point: Vec3::ZERO,
                normal: Vec3::ZERO,
                object: None,
            };
        }

        let point = ray.origin + ray.direction * distance;
        let normal = (point - self.center).normalize();
        Hit {
            distance,
            point,
            normal,
            object: None, // 이 함수에서는 object 필드를 채우지 않습니다. FindClosestCollision에서 채웁니다.
        }
    }

    fn ambient(&self) -> Vec3 {
        self.amb
    }
    fn diffuse(&self) -> Vec3 {
        self.diff
    }
    fn specular(&self) -> Vec3 {
        self.spec
    }
    fn alpha(&self) -> f32 {
        self.alpha
    }
}

// C++: class Light
struct Light {
    pos: Vec3,
}

// C++: class Raytracer
struct Raytracer {
    width: i32,
    height: i32,
    // C++: std::vector<shared_ptr<Object>> objects;
    // Rust: 여러 타입의 Object를 담기 위해 Arc<dyn Object>의 벡터를 사용합니다.
    objects: Vec<Arc<dyn Object>>,
    light: Light,
}

impl Raytracer {
    fn new(width: i32, height: i32) -> Self {
        // C++ 생성자에서 구 3개를 만들고 objects 벡터에 추가하는 로직과 동일합니다.
        let sphere1 = Arc::new(Sphere {
            center: vec3(0.5, 0.0, 0.5),
            radius: 0.4,
            amb: vec3(0.2, 0.2, 0.2),
            diff: vec3(1.0, 0.2, 0.2),
            spec: vec3(0.5, 0.5, 0.5),
            alpha: 10.0,
        });
        let sphere2 = Arc::new(Sphere {
            center: vec3(0.0, 0.0, 1.0),
            radius: 0.4,
            amb: vec3(0.2, 0.2, 0.2),
            diff: vec3(0.2, 1.0, 0.2),
            spec: vec3(0.5, 0.5, 0.5),
            alpha: 10.0,
        });
        let sphere3 = Arc::new(Sphere {
            center: vec3(-0.5, 0.0, 1.5),
            radius: 0.4,
            amb: vec3(0.2, 0.2, 0.2),
            diff: vec3(0.2, 0.2, 1.0),
            spec: vec3(0.5, 0.5, 0.5),
            alpha: 10.0,
        });

        Self {
            width,
            height,
            objects: vec![sphere3, sphere2, sphere1],
            light: Light {
                pos: vec3(0.0, 1.0, -1.0),
            },
        }
    }

    // C++: Hit FindClosestCollision(Ray& ray)
    // 가장 가까운 충돌 지점을 찾는 로직입니다.
    fn find_closest_collision(&self, ray: &Ray) -> Hit {
        let mut closest_d = f32::MAX;
        let mut closest_hit = Hit {
            distance: -1.0,
            point: Vec3::ZERO,
            normal: Vec3::ZERO,
            object: None,
        };

        for object in &self.objects {
            let mut hit = object.check_ray_collision(ray);
            if hit.distance >= 0.0 && hit.distance < closest_d {
                closest_d = hit.distance;
                hit.object = Some(object.clone()); // 충돌한 객체의 참조를 저장합니다.
                closest_hit = hit;
            }
        }
        closest_hit
    }

    fn transform_screen_to_world(&self, pos_screen: Vec2) -> Vec3 {
        let x_scale = 2.0 / (self.width - 1) as f32;
        let y_scale = 2.0 / (self.height - 1) as f32;
        let aspect = self.width as f32 / self.height as f32;

        vec3(
            (pos_screen.x * x_scale - 1.0) * aspect,
            -pos_screen.y * y_scale + 1.0,
            0.0,
        )
    }

    // C++: vec3 traceRay(Ray &ray)
    fn trace_ray(&self, ray: &Ray) -> Vec3 {
        let hit = self.find_closest_collision(ray);

        if let Some(obj) = hit.object {
            let dir_to_light =
                (self.light.pos - hit.point).normalize();
            let diff = hit.normal.dot(dir_to_light).max(0.0);

            let reflect_dir =
                2.0 * hit.normal.dot(dir_to_light) * hit.normal
                    - dir_to_light;
            let specular = (-ray.direction)
                .dot(reflect_dir)
                .max(0.0)
                .powf(obj.alpha());

            obj.ambient()
                + obj.diffuse() * diff
                + obj.specular() * specular
        } else {
            vec3(0.0, 0.0, 0.0)
        }
    }

    // C++: void Render(std::vector<glm::vec4>& pixels)
    fn render(&self, pixels: &mut [Color32]) {
        use rayon::prelude::*;

        // C++: const vec3 eyePos(0.0f, 0.0f, -1.5f);
        let eye_pos = vec3(0.0, 0.0, -1.5);

        pixels.par_iter_mut().enumerate().for_each(|(idx, pixel)| {
            let i = idx % self.width as usize;
            let j = idx / self.width as usize;

            let pos_world = self
                .transform_screen_to_world(vec2(i as f32, j as f32));

            // C++: Ray pixelRay{ pixelPosWorld, normalize(pixelPosWorld - eyePos) };
            // 원근 투영을 위해 시점에서 픽셀 위치를 향하는 광선을 생성합니다.
            let ray_dir = (pos_world - eye_pos).normalize();
            let pixel_ray = Ray {
                origin: eye_pos, // 원근 투영에서는 광선이 시점에서 시작됩니다.
                direction: ray_dir,
            };

            let color_vec = self.trace_ray(&pixel_ray);

            *pixel = Color32::from_rgb(
                (color_vec.x.clamp(0.0, 1.0) * 255.0) as u8,
                (color_vec.y.clamp(0.0, 1.0) * 255.0) as u8,
                (color_vec.z.clamp(0.0, 1.0) * 255.0) as u8,
            );
        });
    }
}

// C++의 Example 클래스와 main 함수의 UI 로직을 TemplateApp으로 통합합니다.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct TemplateApp {
    #[serde(skip)]
    raytracer: Raytracer,

    // UI 컨트롤을 위한 변수들
    light_x: f32,
    light_y: f32,
    light_z: f32,
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
            light_x: raytracer.light.pos.x,
            light_y: raytracer.light.pos.y,
            light_z: raytracer.light.pos.z,
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
        // UI 값 Raytracer에 반영
        self.raytracer.light.pos.x = self.light_x;
        self.raytracer.light.pos.y = self.light_y;
        self.raytracer.light.pos.z = self.light_z;

        egui::SidePanel::left("control_panel").show(ctx, |ui| {
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
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            let width = self.raytracer.width as usize;
            let height = self.raytracer.height as usize;

            let mut pixels: Vec<Color32> =
                vec![
                    consts::DEFAULT_BACKGROUND_COLOR;
                    width * height
                ];

            self.raytracer.render(&mut pixels);

            let image = egui::ColorImage::from_rgba_unmultiplied(
                [width, height],
                bytemuck::cast_slice(&pixels),
            );

            let texture = ctx.load_texture(
                "sphere_canvas",
                image,
                egui::TextureOptions::NEAREST,
            );
            ui.image((texture.id(), texture.size_vec2()));
        });

        ctx.request_repaint();
    }
}
