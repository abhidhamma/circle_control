use eframe::egui;
use egui::{Color32, TextureOptions};
use glam::{Vec2, Vec3, vec2, vec3};
use image::GenericImageView;
use rayon::prelude::*;
use std::any::Any;
use std::sync::Arc;

/*
 * Object 트레잇: 씬(Scene)에 포함될 수 있는 모든 객체의 공통 인터페이스
 * Send + Sync: 여러 스레드에서 안전하게 공유 가능 (rayon 병렬 처리용)
 * Any: 런타임에 타입 정보를 제공하여 다운캐스팅 가능
*/
trait Object: Send + Sync + Any {
    fn check_ray_collision(&self, ray: &Ray) -> Hit;
    fn ambient(&self) -> Vec3;
    fn diffuse(&self) -> Vec3;
    fn specular(&self) -> Vec3;
    fn alpha(&self) -> f32;
    fn amb_texture(&self) -> Option<Arc<Texture>>;
    fn diff_texture(&self) -> Option<Arc<Texture>>;
    fn as_any(&self) -> &dyn Any;
}

// 광선 충돌 정보를 담는 구조체
struct Hit {
    distance: f32,
    point: Vec3,
    normal: Vec3,
    uv: Vec2, // 텍스처 좌표
    object: Option<Arc<dyn Object>>,
}

// 광선을 나타내는 구조체
struct Ray {
    origin: Vec3,
    direction: Vec3,
}

// 구(Sphere)를 나타내는 구조체
struct Sphere {
    center: Vec3,
    radius: f32,
    amb: Vec3,
    diff: Vec3,
    spec: Vec3,
    alpha: f32,
}

// Sphere에 대한 Object 트레잇 구현
impl Object for Sphere {
    fn check_ray_collision(&self, ray: &Ray) -> Hit {
        let oc = ray.origin - self.center;
        let a = ray.direction.length_squared();
        let b = 2.0 * ray.direction.dot(oc);
        let c = oc.length_squared() - self.radius * self.radius;
        let discriminant = b * b - 4.0 * a * c;

        if discriminant < 0.0 {
            return Hit {
                distance: -1.0,
                point: Vec3::ZERO,
                normal: Vec3::ZERO,
                uv: Vec2::ZERO,
                object: None,
            };
        }

        let sqrt_discriminant = discriminant.sqrt();
        let t1 = (-b - sqrt_discriminant) / (2.0 * a);
        let t2 = (-b + sqrt_discriminant) / (2.0 * a);

        let distance = if t1 >= 0.0 {
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
                uv: Vec2::ZERO,
                object: None,
            };
        }

        let point = ray.origin + ray.direction * distance;
        let normal = (point - self.center).normalize();
        Hit {
            distance,
            point,
            normal,
            uv: Vec2::ZERO, // 구는 텍스처 좌표 없음
            object: None,
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
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
    fn amb_texture(&self) -> Option<Arc<Texture>> {
        None
    }
    fn diff_texture(&self) -> Option<Arc<Texture>> {
        None
    }
}

// 삼각형(Triangle)을 나타내는 구조체
struct Triangle {
    v0: Vec3,
    v1: Vec3,
    v2: Vec3,
    uv0: Vec2,
    uv1: Vec2,
    uv2: Vec2,
    amb: Vec3,
    diff: Vec3,
    spec: Vec3,
    alpha: f32,
    amb_texture: Option<Arc<Texture>>,
    diff_texture: Option<Arc<Texture>>,
}

impl Triangle {
    // 광선과 삼각형 교점 찾기
    fn intersect_ray_triangle(
        &self,
        ray: &Ray,
    ) -> Option<(f32, Vec3, Vec3, f32, f32)> {
        let face_normal =
            (self.v1 - self.v0).cross(self.v2 - self.v0).normalize();

        // Backface culling: 삼각형 뒷면은 그리지 않음
        if (-ray.direction).dot(face_normal) < 0.0 {
            return None;
        }

        // 평면과 광선이 거의 평행하면 충돌하지 않음
        if ray.direction.dot(face_normal).abs() < 1e-2 {
            return None;
        }

        // 광선과 평면의 교점 계산
        let t = (self.v0.dot(face_normal)
            - ray.origin.dot(face_normal))
            / ray.direction.dot(face_normal);

        if t < 0.0 {
            return None;
        }

        let point = ray.origin + t * ray.direction;

        // 교점이 삼각형 내부에 있는지 확인
        let cross0 = (point - self.v2).cross(self.v1 - self.v2);
        let cross1 = (point - self.v0).cross(self.v2 - self.v0);
        let cross2 = (self.v1 - self.v0).cross(point - self.v0);

        if cross0.dot(face_normal) < 0.0
            || cross1.dot(face_normal) < 0.0
            || cross2.dot(face_normal) < 0.0
        {
            return None;
        }

        // 무게중심 좌표(Barycentric coordinates) 계산
        let area0 = cross0.length() * 0.5;
        let area1 = cross1.length() * 0.5;
        let _area2 = cross2.length() * 0.5;
        let area_sum =
            (self.v1 - self.v0).cross(self.v2 - self.v0).length()
                * 0.5;

        let w0 = area0 / area_sum;
        let w1 = area1 / area_sum;

        Some((t, point, face_normal, w0, w1))
    }
}

impl Object for Triangle {
    fn check_ray_collision(&self, ray: &Ray) -> Hit {
        if let Some((t, point, normal, w0, w1)) =
            self.intersect_ray_triangle(ray)
        {
            let w2 = 1.0 - w0 - w1;
            let uv = self.uv0 * w0 + self.uv1 * w1 + self.uv2 * w2;
            Hit {
                distance: t,
                point,
                normal,
                uv,
                object: None,
            }
        } else {
            Hit {
                distance: -1.0,
                point: Vec3::ZERO,
                normal: Vec3::ZERO,
                uv: Vec2::ZERO,
                object: None,
            }
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
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
    fn amb_texture(&self) -> Option<Arc<Texture>> {
        self.amb_texture.clone()
    }
    fn diff_texture(&self) -> Option<Arc<Texture>> {
        self.diff_texture.clone()
    }
}

// 사각형(Square)을 나타내는 구조체
struct Square {
    triangle1: Triangle,
    triangle2: Triangle,
}

impl Object for Square {
    fn check_ray_collision(&self, ray: &Ray) -> Hit {
        let hit1 = self.triangle1.check_ray_collision(ray);
        let hit2 = self.triangle2.check_ray_collision(ray);

        if hit1.distance >= 0.0 && hit2.distance >= 0.0 {
            if hit1.distance < hit2.distance {
                hit1
            } else {
                hit2
            }
        } else if hit1.distance >= 0.0 {
            hit1
        } else {
            hit2
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
    fn ambient(&self) -> Vec3 {
        self.triangle1.ambient()
    }
    fn diffuse(&self) -> Vec3 {
        self.triangle1.diffuse()
    }
    fn specular(&self) -> Vec3 {
        self.triangle1.specular()
    }
    fn alpha(&self) -> f32 {
        self.triangle1.alpha()
    }
    fn amb_texture(&self) -> Option<Arc<Texture>> {
        self.triangle1.amb_texture()
    }
    fn diff_texture(&self) -> Option<Arc<Texture>> {
        self.triangle1.diff_texture()
    }
}

// 점 광원(Point Light)을 나타내는 구조체
struct Light {
    pos: Vec3,
}

// 텍스처 데이터를 담는 구조체
struct Texture {
    width: u32,
    height: u32,
    channels: u32,
    image: Vec<u8>,
}

impl Texture {
    fn from_dynamic_image(img: image::DynamicImage) -> Self {
        let (width, height) = img.dimensions();
        let image_data = img.to_rgba8().into_raw();

        Self {
            width,
            height,
            channels: 4,
            image: image_data,
        }
    }

    fn get_wrapped(&self, mut i: i32, mut j: i32) -> Vec3 {
        i %= self.width as i32;
        j %= self.height as i32;
        if i < 0 {
            i += self.width as i32;
        }
        if j < 0 {
            j += self.height as i32;
        }

        let idx = ((j as u32 * self.width + i as u32) * self.channels)
            as usize;
        let r = self.image[idx] as f32 / 255.0;
        let g = self.image[idx + 1] as f32 / 255.0;
        let b = self.image[idx + 2] as f32 / 255.0;
        vec3(r, g, b)
    }

    fn interpolate_bilinear(
        &self,
        dx: f32,
        dy: f32,
        c00: Vec3,
        c10: Vec3,
        c01: Vec3,
        c11: Vec3,
    ) -> Vec3 {
        let a = c00.lerp(c10, dx);
        let b = c01.lerp(c11, dx);
        a.lerp(b, dy)
    }

    fn sample_point(&self, uv: Vec2) -> Vec3 {
        let xy = uv * vec2(self.width as f32, self.height as f32)
            - vec2(0.5, 0.5);
        let i = xy.x.round() as i32;
        let j = xy.y.round() as i32;
        self.get_wrapped(i, j)
    }

    fn sample_linear(&self, uv: Vec2) -> Vec3 {
        let xy = uv * vec2(self.width as f32, self.height as f32)
            - vec2(0.5, 0.5);
        let i = xy.x.floor() as i32;
        let j = xy.y.floor() as i32;
        let dx = xy.x - i as f32;
        let dy = xy.y - j as f32;

        let c00 = self.get_wrapped(i, j);
        let c10 = self.get_wrapped(i + 1, j);
        let c01 = self.get_wrapped(i, j + 1);
        let c11 = self.get_wrapped(i + 1, j + 1);

        self.interpolate_bilinear(dx, dy, c00, c10, c01, c11)
    }
}

// 레이 트레이서
struct Raytracer {
    width: i32,
    height: i32,
    objects: Vec<Arc<dyn Object>>,
    light: Light,
}

impl Raytracer {
    fn new(width: i32, height: i32) -> Self {
        let sphere1 = Arc::new(Sphere {
            center: vec3(1.0, 0.0, 1.5),
            radius: 0.8,
            amb: vec3(0.2, 0.2, 0.2),
            diff: vec3(1.0, 0.2, 0.2),
            spec: vec3(0.5, 0.5, 0.5),
            alpha: 10.0,
        });

        let square = Arc::new(Square {
            triangle1: Triangle {
                v0: vec3(-2.0, 2.0, 2.0),
                v1: vec3(2.0, 2.0, 2.0),
                v2: vec3(2.0, -2.0, 2.0),
                uv0: vec2(0.0, 0.0),
                uv1: vec2(1.0, 0.0),
                uv2: vec2(1.0, 1.0),
                amb: vec3(0.2, 0.2, 0.2),
                diff: vec3(1.0, 1.0, 1.0),
                spec: vec3(0.0, 0.0, 0.0),
                alpha: 10.0,
                amb_texture: None,
                diff_texture: None,
            },
            triangle2: Triangle {
                v0: vec3(-2.0, 2.0, 2.0),
                v1: vec3(2.0, -2.0, 2.0),
                v2: vec3(-2.0, -2.0, 2.0),
                uv0: vec2(0.0, 0.0),
                uv1: vec2(1.0, 1.0),
                uv2: vec2(0.0, 1.0),
                amb: vec3(0.2, 0.2, 0.2),
                diff: vec3(1.0, 1.0, 1.0),
                spec: vec3(0.0, 0.0, 0.0),
                alpha: 10.0,
                amb_texture: None,
                diff_texture: None,
            },
        });

        let objects: Vec<Arc<dyn Object>> = vec![sphere1, square];

        Self {
            width,
            height,
            objects,
            light: Light {
                pos: vec3(0.0, 1.0, 0.5),
            },
        }
    }

    fn find_closest_collision(&self, ray: &Ray) -> Hit {
        let mut closest_d = f32::MAX;
        let mut closest_hit = Hit {
            distance: -1.0,
            point: Vec3::ZERO,
            normal: Vec3::ZERO,
            uv: Vec2::ZERO,
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

    fn trace_ray(&self, ray: &Ray) -> Vec3 {
        let hit = self.find_closest_collision(ray);

        if let Some(obj) = hit.object {
            let mut color;

            // Ambient
            if let Some(tex) = obj.amb_texture() {
                color = obj.ambient() * tex.sample_linear(hit.uv);
            } else {
                color = obj.ambient();
            }

            let dir_to_light =
                (self.light.pos - hit.point).normalize();
            let diff_intensity =
                hit.normal.dot(dir_to_light).max(0.0);

            // Diffuse
            if let Some(tex) = obj.diff_texture() {
                color += obj.diffuse()
                    * diff_intensity
                    * tex.sample_linear(hit.uv);
            } else {
                color += obj.diffuse() * diff_intensity;
            }

            // Specular
            let reflect_dir =
                2.0 * hit.normal.dot(dir_to_light) * hit.normal
                    - dir_to_light;
            let specular = (-ray.direction)
                .dot(reflect_dir)
                .max(0.0)
                .powf(obj.alpha());
            color += obj.specular() * specular;

            color
        } else {
            vec3(0.0, 0.0, 0.0)
        }
    }

    // 슈퍼샘플링을 위한 재귀 함수
    fn trace_ray_2x2(
        &self,
        eye_pos: Vec3,
        pixel_pos: Vec3,
        dx: f32,
        recursive_level: i32,
    ) -> Vec3 {
        if recursive_level == 0 {
            let ray = Ray {
                origin: pixel_pos,
                direction: (pixel_pos - eye_pos).normalize(),
            };
            return self.trace_ray(&ray);
        }

        let sub_dx = 0.5 * dx;
        let mut pixel_color = Vec3::ZERO;

        // 현재 픽셀을 2x2 서브픽셀로 나누어 각각 광선을 쏨
        for j in 0..2 {
            for i in 0..2 {
                let sub_pos = vec3(
                    pixel_pos.x + (i as f32 - 0.5) * sub_dx,
                    pixel_pos.y + (j as f32 - 0.5) * sub_dx,
                    pixel_pos.z,
                );
                pixel_color += self.trace_ray_2x2(
                    eye_pos,
                    sub_pos,
                    sub_dx,
                    recursive_level - 1,
                );
            }
        }

        pixel_color * 0.25 // 4개 서브픽셀 색상의 평균
    }

    fn transform_screen_to_world(&self, pos_screen: Vec2) -> Vec3 {
        let x_scale = 2.0 / self.width as f32;
        let y_scale = 2.0 / self.height as f32;
        let aspect = self.width as f32 / self.height as f32;

        vec3(
            (pos_screen.x * x_scale - 1.0) * aspect,
            -pos_screen.y * y_scale + 1.0,
            0.0,
        )
    }

    fn render(&mut self, pixels: &mut [Color32]) {
        let eye_pos = vec3(0.0, 0.0, -1.5);

        pixels.par_iter_mut().enumerate().for_each(|(idx, pixel)| {
            let i = idx % self.width as usize;
            let j = idx / self.width as usize;

            let pos_world = self
                .transform_screen_to_world(vec2(i as f32, j as f32));

            // 슈퍼샘플링 적용-
            // 1. 픽셀당 광선 하나(슈퍼샘플링 적용X)
            let ray_dir = (pos_world - eye_pos).normalize();
            let pixel_ray = Ray {
                origin: eye_pos,
                direction: ray_dir,
            };
            // let color_vec = self.trace_ray(&pixel_ray);

            // 2. 2x2 슈퍼샘플링
            // (픽셀중심좌표를 기준으로 픽셀을 네개로 분할해서 광선을 쏜 뒤 평균값으로 렌더링)
            let dx = 2.0 / self.height as f32;
            let color_vec =
                self.trace_ray_2x2(eye_pos, pos_world, dx, 1);

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

    #[serde(skip)]
    is_first_frame: bool,
}

impl Default for TemplateApp {
    fn default() -> Self {
        Self {
            raytracer: Raytracer::new(1280 / 2, 720 / 2),
            is_first_frame: true,
        }
    }
}

impl TemplateApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Default::default()
    }
}

impl eframe::App for TemplateApp {
    fn save(&mut self, _storage: &mut dyn eframe::Storage) {}

    fn update(
        &mut self,
        ctx: &egui::Context,
        _frame: &mut eframe::Frame,
    ) {
        egui::CentralPanel::default().show(ctx, |ui| {
            let available_size = ui.available_size();
            /* 렌더링 해상도를 창 크기에 맞추지 않고, 생성자에서 설정한 값으로 고정함 */
            /* self.raytracer.width = available_size.x as i32; */
            /* self.raytracer.height = available_size.y as i32; */

            let width = self.raytracer.width as usize;
            let height = self.raytracer.height as usize;

            /* 첫 프레임에만 렌더링을 수행하여 결과를 텍스처에 저장함 */
            if self.is_first_frame {
                let mut pixels: Vec<Color32> = vec![Color32::BLACK; width * height];
                self.raytracer.render(&mut pixels);

                let image = egui::ColorImage::from_rgba_unmultiplied(
                    [width, height],
                    bytemuck::cast_slice(&pixels),
                );

                /* 렌더링된 이미지를 egui 컨텍스트 메모리에 저장 */
                ctx.memory_mut(|mem| {
                    mem.data.insert_temp(egui::Id::new("raytrace_texture"), image)
                });

                self.is_first_frame = false;
            }

            /* 매 프레임 저장된 이미지를 불러와 화면에 그림 */
            if let Some(texture) =
                ctx.memory(|mem| mem.data.get_temp::<egui::ColorImage>(egui::Id::new("raytrace_texture")))
            {
                let texture_handle = ctx.load_texture(
                    "raytrace_canvas",
                    texture.clone(),
                    TextureOptions::NEAREST,
                );
                /* 이미지를 UI에 맞게 크기를 조절하여 표시 */
               let image = egui::Image::new(&texture_handle)
                    .fit_to_original_size(1.0) // 원본 종횡비 유지
                    .shrink_to_fit(); // UI 공간에 맞게 축소
                ui.centered_and_justified(|ui| {
                    ui.add(image);
                });
            }
        });
    }
}
