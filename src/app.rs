use egui::{Color32, Slider, Vec2, vec2};
use glam::{Vec3, vec3};
use std::sync::Arc;

// trait 동적 디스패치를 위해 Box<dyn Object>, Arc<dyn Object> 사용
trait Object: Send + Sync {
    fn check_ray_collision(&self, ray: &Ray) -> Hit;
    fn ambient(&self) -> Vec3;
    fn diffuse(&self) -> Vec3;
    fn specular(&self) -> Vec3;
    fn alpha(&self) -> f32;
}

// Object, Sphere, Triangle, Light, Hit, Ray struct
struct Hit {
    distance: f32,
    point: Vec3,
    normal: Vec3,
    // 충돌한 객체의 재질 정보
    object: Option<Arc<dyn Object>>,
}

struct Ray {
    origin: Vec3,
    direction: Vec3,
}

struct Sphere {
    center: Vec3,
    radius: f32,
    amb: Vec3,
    diff: Vec3,
    spec: Vec3,
    alpha: f32,
}

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
            object: None, // FindClosestCollision에서 object 넣기
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

// 삼각형 struct
struct Triangle {
    v0: Vec3,
    v1: Vec3,
    v2: Vec3,
    amb: Vec3,
    diff: Vec3,
    spec: Vec3,
    alpha: f32,
}

impl Triangle {
    // 광선과 삼각형의 교점을 찾는 함수
    fn intersect_ray_triangle(
        &self,
        ray: &Ray,
    ) -> Option<(f32, Vec3, Vec3)> {
        // 광원과 삼각형 사이의 거리를 구하기 위해
        // 삼각형 평면의 수직벡터 찾기
        let edge1 = self.v1 - self.v0;
        let edge2 = self.v2 - self.v0;
        let face_normal = edge1.cross(edge2).normalize();

        // 뒷면제거(Back-face culling): 두번그릴 필요 없으니까
        // 광선이 삼각형의 뒷면에서 온다면 충돌하지 않은 것으로 처리
        if ray.direction.dot(face_normal) > 0.0 {
            return None;
        }

        // 광선과 평면이 거의 평행한 경우, 충돌하지 않음
        // (0으로 나누면 에러나는것 방지)
        if ray.direction.dot(face_normal).abs() < 1e-6 {
            return None;
        }

        // 1. 광선과 삼각형이 있는 평면이 만나는 점과의 거리 t 찾기
        /*
           시점에서 쏘아진 광선이 삼각형 위의 점 p를 구하기

           평면의 방정식: (p - v0) · n = 0
           광선의 방정식: p = origin + t * direction
           p 치환: (origin + t * direction - v0) · n = 0
           t만 미지수이므로 t에 대한 식으로 바꾸기
           t * (direction · n) = (v0 - origin) · n
           t = ((v0 - origin) · n) / (direction · n)
        */

        let t = (self.v0 - ray.origin).dot(face_normal)
            / ray.direction.dot(face_normal);

        // 광선의 시작점(눈)보다 뒤에 삼각형이 있다면 충돌하지 않은 것
        if t < 0.0 {
            return None;
        }

        // 2. 교점이 삼각형 내부에 있는지 확인(Inside-Outside Test)
        let point = ray.origin + t * ray.direction;

        // 각 변에서 교점으로 향하는 벡터를 계산
        // 오른손법칙에 의해 삼각형 내부에 있다면 수직벡터는 0보다 큼
        let c0 = point - self.v0;
        let c1 = point - self.v1;
        let c2 = point - self.v2;

        // 벡터가 face_normal과 같은 방향인지 확인
        // (v1-v0) x (point-v0)
        if face_normal.dot(edge1.cross(c0)) < 0.0 {
            return None;
        }
        // (v2-v1) x (point-v1)
        if face_normal.dot((self.v2 - self.v1).cross(c1)) < 0.0 {
            return None;
        }
        // (v0-v2) x (point-v2)
        if face_normal.dot((self.v0 - self.v2).cross(c2)) < 0.0 {
            return None;
        }

        // 모든 테스트를 통과하면 교점은 삼각형 내부에 있음
        Some((t, point, face_normal))
    }
}

impl Object for Triangle {
    fn check_ray_collision(&self, ray: &Ray) -> Hit {
        if let Some((t, point, normal)) =
            self.intersect_ray_triangle(ray)
        {
            Hit {
                distance: t,
                point,
                normal,
                object: None,
            }
        } else {
            Hit {
                distance: -1.0,
                point: Vec3::ZERO,
                normal: Vec3::ZERO,
                object: None,
            }
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

struct Light {
    pos: Vec3,
}

struct Raytracer {
    width: i32,
    height: i32,
    objects: Vec<Arc<dyn Object>>,
    light: Light,
}

impl Raytracer {
    fn new(width: i32, height: i32) -> Self {
        let sphere1 = Arc::new(Sphere {
            center: vec3(0.6, 0.0, 0.5),
            radius: 0.4,
            amb: vec3(0.1, 0.1, 0.1),
            diff: vec3(1.0, 0.1, 0.1),
            spec: vec3(1.0, 1.0, 1.0),
            alpha: 50.0,
        });

        let triangle1 = Arc::new(Triangle {
            v0: vec3(-2.0, -2.0, 2.0),
            v1: vec3(-2.0, 2.0, 2.0),
            v2: vec3(2.0, 2.0, 2.0),
            amb: vec3(0.2, 0.2, 0.2),
            diff: vec3(0.5, 0.5, 0.5),
            spec: vec3(0.5, 0.5, 0.5),
            alpha: 5.0,
        });

        Self {
            width,
            height,
            objects: vec![sphere1, triangle1],
            light: Light {
                pos: vec3(0.0, 1.0, -1.0),
            },
        }
    }

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
                hit.object = Some(object.clone());
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

    fn render(&self, pixels: &mut [Color32]) {
        use rayon::prelude::*;

        let eye_pos = vec3(0.0, 0.0, -1.5);

        pixels.par_iter_mut().enumerate().for_each(|(idx, pixel)| {
            let i = idx % self.width as usize;
            let j = idx / self.width as usize;

            let pos_world = self
                .transform_screen_to_world(vec2(i as f32, j as f32));
            let ray_dir = (pos_world - eye_pos).normalize();
            let pixel_ray = Ray {
                origin: eye_pos,
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
            let available_size = ui.available_size();
            self.raytracer.width = available_size.x as i32;
            self.raytracer.height = available_size.y as i32;

            let width = self.raytracer.width as usize;
            let height = self.raytracer.height as usize;

            let mut pixels: Vec<Color32> = vec![
                    consts::DEFAULT_BACKGROUND_COLOR;
                    width * height
                ];

            self.raytracer.render(&mut pixels);

            let image = egui::ColorImage::from_rgba_unmultiplied(
                [width, height],
                bytemuck::cast_slice(&pixels),
            );

            let texture = ctx.load_texture(
                "raytrace_canvas",
                image,
                egui::TextureOptions::NEAREST,
            );
            ui.image((texture.id(), texture.size_vec2()));
        });

        ctx.request_repaint();
    }
}
