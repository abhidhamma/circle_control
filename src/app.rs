use eframe::egui;
use egui::{Color32, TextureOptions};
use glam::{Vec2, Vec3, vec2, vec3};
use image::GenericImageView;
use rayon::prelude::*;
use std::sync::Arc;

/*
 * Object 트레잇: 씬(Scene)에 포함될 수 있는 모든 객체의 공통 인터페이스
 * Send + Sync: 여러 스레드에서 안전하게 공유 가능 (rayon 병렬 처리용)
*/
trait Object: Send + Sync {
    fn check_ray_collision(&self, ray: &Ray) -> Hit;
    fn ambient(&self) -> Vec3;
    fn diffuse(&self) -> Vec3;
    fn specular(&self) -> Vec3;
    fn alpha(&self) -> f32;
    fn reflection(&self) -> f32;
    fn transparency(&self) -> f32;
    fn amb_texture(&self) -> Option<Arc<Texture>>;
    fn diff_texture(&self) -> Option<Arc<Texture>>;
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
    reflection: f32,
    transparency: f32,
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
    fn reflection(&self) -> f32 {
        self.reflection
    }
    fn transparency(&self) -> f32 {
        self.transparency
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
        let area_sum =
            (self.v1 - self.v0).cross(self.v2 - self.v0).length()
                * 0.5;

        let w0 = area0 / area_sum;
        let w1 = area1 / area_sum;

        Some((t, point, face_normal, w0, w1))
    }
}

// 사각형(Square)을 나타내는 구조체
struct Square {
    triangle1: Triangle,
    triangle2: Triangle,
    amb: Vec3,
    diff: Vec3,
    spec: Vec3,
    alpha: f32,
    reflection: f32,
    transparency: f32,
    amb_texture: Option<Arc<Texture>>,
    diff_texture: Option<Arc<Texture>>,
}

impl Object for Square {
    fn check_ray_collision(&self, ray: &Ray) -> Hit {
        let hit1 = self.triangle1.intersect_ray_triangle(ray);
        let hit2 = self.triangle2.intersect_ray_triangle(ray);

        match (hit1, hit2) {
            (Some(h1), Some(h2)) => {
                if h1.0 < h2.0 {
                    let (t, point, normal, w0, w1) = h1;
                    let w2 = 1.0 - w0 - w1;
                    let uv = self.triangle1.uv0 * w0
                        + self.triangle1.uv1 * w1
                        + self.triangle1.uv2 * w2;
                    Hit {
                        distance: t,
                        point,
                        normal,
                        uv,
                        object: None,
                    }
                } else {
                    let (t, point, normal, w0, w1) = h2;
                    let w2 = 1.0 - w0 - w1;
                    let uv = self.triangle2.uv0 * w0
                        + self.triangle2.uv1 * w1
                        + self.triangle2.uv2 * w2;
                    Hit {
                        distance: t,
                        point,
                        normal,
                        uv,
                        object: None,
                    }
                }
            }
            (Some(h1), None) => {
                let (t, point, normal, w0, w1) = h1;
                let w2 = 1.0 - w0 - w1;
                let uv = self.triangle1.uv0 * w0
                    + self.triangle1.uv1 * w1
                    + self.triangle1.uv2 * w2;
                Hit {
                    distance: t,
                    point,
                    normal,
                    uv,
                    object: None,
                }
            }
            (None, Some(h2)) => {
                let (t, point, normal, w0, w1) = h2;
                let w2 = 1.0 - w0 - w1;
                let uv = self.triangle2.uv0 * w0
                    + self.triangle2.uv1 * w1
                    + self.triangle2.uv2 * w2;
                Hit {
                    distance: t,
                    point,
                    normal,
                    uv,
                    object: None,
                }
            }
            (None, None) => Hit {
                distance: -1.0,
                point: Vec3::ZERO,
                normal: Vec3::ZERO,
                uv: Vec2::ZERO,
                object: None,
            },
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
    fn reflection(&self) -> f32 {
        self.reflection
    }
    fn transparency(&self) -> f32 {
        self.transparency
    }
    fn amb_texture(&self) -> Option<Arc<Texture>> {
        self.amb_texture.clone()
    }
    fn diff_texture(&self) -> Option<Arc<Texture>> {
        self.diff_texture.clone()
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
            center: vec3(0.0, -0.1, 1.5),
            radius: 1.0,
            amb: vec3(0.1, 0.1, 0.1),
            diff: vec3(1.0, 0.0, 0.0),
            spec: vec3(1.0, 1.0, 1.0),
            alpha: 10.0,
            reflection: 0.5,
            transparency: 0.0,
        });

        let sphere2 = Arc::new(Sphere {
            center: vec3(1.2, -0.1, 0.5),
            radius: 0.4,
            amb: Vec3::ZERO,
            diff: vec3(0.0, 0.0, 1.0),
            spec: vec3(1.0, 1.0, 1.0),
            alpha: 50.0,
            reflection: 0.5,
            transparency: 0.0,
        });

        let ground_texture_bytes =
            include_bytes!("../assets/shadertoy_abstract1.jpg");
        let ground_image =
            image::load_from_memory(ground_texture_bytes).unwrap();
        let ground_texture =
            Arc::new(Texture::from_dynamic_image(ground_image));

        let ground = Arc::new(Square {
            triangle1: Triangle {
                v0: vec3(-10.0, -1.2, 0.0),
                v1: vec3(-10.0, -1.2, 10.0),
                v2: vec3(10.0, -1.2, 10.0),
                uv0: vec2(0.0, 0.0),
                uv1: vec2(1.0, 0.0),
                uv2: vec2(1.0, 1.0),
            },
            triangle2: Triangle {
                v0: vec3(-10.0, -1.2, 0.0),
                v1: vec3(10.0, -1.2, 10.0),
                v2: vec3(10.0, -1.2, 0.0),
                uv0: vec2(0.0, 0.0),
                uv1: vec2(1.0, 1.0),
                uv2: vec2(0.0, 1.0),
            },
            amb: vec3(1.0, 1.0, 1.0),
            diff: vec3(1.0, 1.0, 1.0),
            spec: vec3(1.0, 1.0, 1.0),
            alpha: 10.0,
            reflection: 0.5,
            transparency: 0.0,
            amb_texture: Some(ground_texture.clone()),
            diff_texture: Some(ground_texture),
        });

        let objects: Vec<Arc<dyn Object>> =
            vec![sphere1, sphere2, ground];

        Self {
            width,
            height,
            objects,
            light: Light {
                pos: vec3(0.0, 0.5, -0.5),
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

    fn trace_ray(&self, ray: &Ray, recurse_level: i32) -> Vec3 {
        // 빛의 반사 횟수가 level보다 작으면 검은색 반환
        if recurse_level < 0 {
            return Vec3::ZERO;
        }

        // 광선과 가장 가까운 물체와의 충돌 정보 찾기
        let hit = self.find_closest_collision(ray);

        // 충돌한 물체가 있으면 phong shading, 다른물체로의 반사 계산
        if let Some(obj) = hit.object {
            // 초기화
            let mut color = Vec3::ZERO;

            // 주변광, 확산광, 반사광
            let dir_to_light =
                (self.light.pos - hit.point).normalize();

            let mut phong_color = Vec3::ZERO;
            let diff = hit.normal.dot(dir_to_light).max(0.0);
            let reflect_dir =
                2.0 * hit.normal * hit.normal.dot(dir_to_light)
                    - dir_to_light;
            let specular = (-ray.direction)
                .dot(reflect_dir)
                .max(0.0)
                .powf(obj.alpha());

            if let Some(tex) = obj.amb_texture() {
                phong_color +=
                    obj.ambient() * tex.sample_linear(hit.uv);
            } else {
                phong_color += obj.ambient();
            }

            if let Some(tex) = obj.diff_texture() {
                phong_color +=
                    diff * obj.diffuse() * tex.sample_linear(hit.uv);
            } else {
                phong_color += diff * obj.diffuse();
            }

            phong_color += obj.specular() * specular;

            color += phong_color
                * (1.0 - obj.reflection() - obj.transparency());

            // 다른 물체로의 반사광
            if obj.reflection() > 0.0 {
                // 반사광 계산과 동일하게 (R = I - 2*dot(I,N)*N)을 이용해 반사 광선 방향 계산
                let reflected_direction = (ray.direction
                    - 2.0
                        * ray.direction.dot(hit.normal)
                        * hit.normal)
                    .normalize();
                // shadow acne(부동소수점 오차로 인한 오류) 방지
                let reflection_ray = Ray {
                    origin: hit.point + reflected_direction * 1e-4,
                    direction: reflected_direction,
                };
                // 반사광을 재귀적으로 추적하고 결과값에 반사율을 곱해 최종 색상에 더하기
                color += self
                    .trace_ray(&reflection_ray, recurse_level - 1)
                    * obj.reflection();
            }

            color
        } else {
            Vec3::ZERO
        }
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
            let ray_dir = (pos_world - eye_pos).normalize();
            let pixel_ray = Ray {
                origin: pos_world,
                direction: ray_dir,
            };
            let color_vec = self.trace_ray(&pixel_ray, 5);

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
            //해상도
            raytracer: Raytracer::new(1280, 720),
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
            let width = self.raytracer.width as usize;
            let height = self.raytracer.height as usize;

            if self.is_first_frame {
                let mut pixels: Vec<Color32> =
                    vec![Color32::BLACK; width * height];
                self.raytracer.render(&mut pixels);

                let image = egui::ColorImage::from_rgba_unmultiplied(
                    [width, height],
                    bytemuck::cast_slice(&pixels),
                );

                ctx.memory_mut(|mem| {
                    mem.data.insert_temp(
                        egui::Id::new("raytrace_texture"),
                        image,
                    )
                });

                self.is_first_frame = false;
            }

            if let Some(texture) = ctx.memory(|mem| {
                mem.data.get_temp::<egui::ColorImage>(egui::Id::new(
                    "raytrace_texture",
                ))
            }) {
                let texture_handle = ctx.load_texture(
                    "raytrace_canvas",
                    texture.clone(),
                    TextureOptions::NEAREST,
                );
                let image = egui::Image::new(&texture_handle)
                    .fit_to_original_size(1.0)
                    .shrink_to_fit();
                ui.centered_and_justified(|ui| {
                    ui.add(image);
                });
            }
        });
    }
}
