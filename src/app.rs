use eframe::egui;
use egui::{Color32, TextureOptions};
use glam::{Vec2, Vec3, vec2, vec3};
use image::GenericImageView;
use std::any::Any;
use std::sync::Arc;

/*
 * C++의 Object 클래스 -> Rust의 Object 트레잇
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

// C++ Hit -> Rust Hit
struct Hit {
    distance: f32,
    point: Vec3,
    normal: Vec3,
    uv: Vec2, // 텍스처 좌표
    object: Option<Arc<dyn Object>>,
}

// C++ Ray -> Rust Ray
struct Ray {
    origin: Vec3,
    direction: Vec3,
}

// C++ Sphere -> Rust Sphere
struct Sphere {
    center: Vec3,
    radius: f32,
    amb: Vec3,
    diff: Vec3,
    spec: Vec3,
    alpha: f32,
}

// Sphere의 Object 트레잇 구현
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
                uv: Vec2::ZERO,
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

// C++ Triangle -> Rust Triangle
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

        if (-ray.direction).dot(face_normal) < 0.0 {
            return None; // Backface culling
        }

        if ray.direction.dot(face_normal).abs() < 1e-2 {
            return None;
        }

        let t = (self.v0.dot(face_normal)
            - ray.origin.dot(face_normal))
            / ray.direction.dot(face_normal);

        if t < 0.0 {
            return None;
        }

        let point = ray.origin + t * ray.direction;

        let cross0 = (point - self.v2).cross(self.v1 - self.v2);
        let cross1 = (point - self.v0).cross(self.v2 - self.v0);
        let cross2 = (self.v1 - self.v0).cross(point - self.v0);

        if cross0.dot(face_normal) < 0.0
            || cross1.dot(face_normal) < 0.0
            || cross2.dot(face_normal) < 0.0
        {
            return None;
        }

        let area0 = cross0.length() * 0.5;
        let area1 = cross1.length() * 0.5;
        let area2 = cross2.length() * 0.5;
        let area_sum = area0 + area1 + area2;

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

// C++ Square -> Rust Square
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

// C++ Light -> Rust Light
struct Light {
    pos: Vec3,
}

// C++ Texture -> Rust Texture
struct Texture {
    width: u32,
    height: u32,
    channels: u32,
    image: Vec<u8>,
}

impl Texture {
    // image-rs의 DynamicImage로부터 Texture를 생성하는 함수
    fn from_dynamic_image(img: image::DynamicImage) -> Self {
        let (width, height) = img.dimensions();
        let image_data = img.to_rgba8().into_raw(); // RGBA8로 통일

        Self {
            width,
            height,
            channels: 4, // RGBA
            image: image_data,
        }
    }

    /*
     * i, j 좌표 픽셀 색상 가져오기. 범위를 벗어나면 가장 가까운 색상으로 clamp.
     * clamp함수는 최대값과 최소값을 제한.
     * - 최대값보다 클때 -> 최대값 리턴
     * - 최대값보다 작거나 같고 최소값보다 크거나 같을때 -> 현재값 리턴
     * - 최소값보다 작을때 -> 최소값 리턴
     */
    fn get_clamped(&self, i: i32, j: i32) -> Vec3 {
        let i = i.clamp(0, self.width as i32 - 1) as u32;
        let j = j.clamp(0, self.height as i32 - 1) as u32;

        let idx = ((j * self.width + i) * self.channels) as usize;
        let r = self.image[idx] as f32 / 255.0;
        let g = self.image[idx + 1] as f32 / 255.0;
        let b = self.image[idx + 2] as f32 / 255.0;
        vec3(r, g, b)
    }

    // i, j 좌표의 픽셀 색상을 가져옴. 범위를 벗어나면 반복(wrapping)시킴.
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

    /// 두 개의 차원에서 수행되는 선형 보간 (Bilinear Interpolation)
    fn interpolate_bilinear(
        &self,
        dx: f32,
        dy: f32,
        c00: Vec3,
        c10: Vec3,
        c01: Vec3,
        c11: Vec3,
    ) -> Vec3 {
        let a = c00 * (1.0 - dx) + c10 * dx;
        let b = c01 * (1.0 - dx) + c11 * dx;
        a * (1.0 - dy) + b * dy
    }

    /// Point Sampling (Nearest-neighbor sampling)
    /// 색을 해당 좌표에서 가장 가까운 픽셀의 색으로 결정.
    fn sample_point(&self, uv: Vec2) -> Vec3 {
        /*
         * 1. 텍스처 좌표(uv): [0.0, 1.0] x [0.0, 1.0]
         * 2. 이미지 좌표(xy): [-0.5, width - 0.5] x [-0.5, height - 0.5]
         * 3. 배열 인덱스(ij): [0, width-1] x [0, height-1]
         */

        // 1 -> 2: uv 좌표계를 이미지 좌표계로 변환
        // 이미지에서 좌표란 한 점이고 이 점은 이미지 픽셀의 가운데 저장되어있다고 가정.
        // 따라서 좌표의 범위를 픽셀 크기만큼 상하좌우로 확장.
        let xy = uv * vec2(self.width as f32, self.height as f32)
            - vec2(0.5, 0.5);

        // 2 -> 3: 가장 가까운 정수 인덱스 찾기
        // round 연산 사용. 예: (0.3, 1) -> (0, 1), (0.7, 1) -> (1, 1)
        let i = xy.x.round() as i32;
        let j = xy.y.round() as i32;

        self.get_clamped(i, j)
    }

    /// Linear Sampling (Bilinear filtering)
    /// 색을 해당 좌표에서 가장 가까운 네 픽셀의 색을 거리에 따라 섞어서 결정.
    fn sample_linear(&self, uv: Vec2) -> Vec3 {
        // 1 -> 2: uv 좌표계를 이미지 좌표계로 변환
        let xy = uv * vec2(self.width as f32, self.height as f32)
            - vec2(0.5, 0.5);

        // 2 -> 3: 현재 좌표가 속한 픽셀의 좌측 하단 정수 인덱스 찾기
        // floor 함수 사용.
        let i = xy.x.floor() as i32;
        let j = xy.y.floor() as i32;

        // 보간에 사용할 가중치 계산
        let dx = xy.x - i as f32;
        let dy = xy.y - j as f32;

        // 주변 4개 픽셀 색상 가져오기
        let c00 = self.get_clamped(i, j);
        let c10 = self.get_clamped(i + 1, j);
        let c01 = self.get_clamped(i, j + 1);
        let c11 = self.get_clamped(i + 1, j + 1);

        // 선형 보간을 세 번 수행하여 최종 색상 계산
        self.interpolate_bilinear(dx, dy, c00, c10, c01, c11)
    }
}

// C++ Raytracer -> Rust Raytracer
struct Raytracer {
    width: i32,
    height: i32,
    objects: Vec<Arc<dyn Object>>,
    light: Light,
}

impl Raytracer {
    // 생성자가 이제 텍스처를 직접 받음
    fn new(
        width: i32,
        height: i32,
        texture: Option<image::DynamicImage>,
    ) -> Self {
        let sphere1 = Arc::new(Sphere {
            center: vec3(1.0, 0.0, 1.5),
            radius: 0.4,
            amb: vec3(0.2, 0.2, 0.2),
            diff: vec3(1.0, 0.2, 0.2),
            spec: vec3(0.5, 0.5, 0.5),
            alpha: 10.0,
        });

        // 텍스처가 로드되었을 때만 Square를 생성
        let mut objects: Vec<Arc<dyn Object>> = vec![sphere1];
        if let Some(img) = texture {
            let image_texture =
                Arc::new(Texture::from_dynamic_image(img));

            let square = Arc::new(Square {
                triangle1: Triangle {
                    v0: vec3(-2.0, 2.0, 2.0),
                    v1: vec3(2.0, 2.0, 2.0),
                    v2: vec3(2.0, -2.0, 2.0),
                    uv0: vec2(0.0, 0.0),
                    uv1: vec2(1.0, 0.0),
                    uv2: vec2(1.0, 1.0),
                    amb: vec3(0.0, 0.0, 0.0),
                    diff: vec3(1.0, 1.0, 1.0),
                    spec: vec3(0.0, 0.0, 0.0),
                    alpha: 10.0,
                    amb_texture: Some(image_texture.clone()),
                    diff_texture: Some(image_texture.clone()),
                },
                triangle2: Triangle {
                    v0: vec3(-2.0, 2.0, 2.0),
                    v1: vec3(2.0, -2.0, 2.0),
                    v2: vec3(-2.0, -2.0, 2.0),
                    uv0: vec2(0.0, 0.0),
                    uv1: vec2(1.0, 1.0),
                    uv2: vec2(0.0, 1.0),
                    amb: vec3(0.0, 0.0, 0.0),
                    diff: vec3(1.0, 1.0, 1.0),
                    spec: vec3(0.0, 0.0, 0.0),
                    alpha: 10.0,
                    amb_texture: Some(image_texture.clone()),
                    diff_texture: Some(image_texture.clone()),
                },
            });
            objects.push(square);
        }

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
                /*
                 * 텍스처링: 모델 하나를 정교하게 만드는 대신에 이미지를 덧붙여서 아주 자세한 모델인것 처럼 렌더링.
                 * 폴리곤들을 쓰는것보다 도형에 사진을 덧씌우는게 훨씬 빠름.
                 */
                // color = obj.ambient() * tex.sample_point(hit.uv); // Point Sampling
                color = obj.ambient() * tex.sample_linear(hit.uv); // Linear Sampling
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

    #[serde(skip)]
    is_first_frame: bool,
}

impl Default for TemplateApp {
    fn default() -> Self {
        // 컴파일 시점에 이미지 파일을 실행 파일에 포함시킴
        let image_bytes = include_bytes!("../assets/rupi.jpg");
        // 메모리 상의 바이트 데이터로부터 이미지를 로드
        let image = image::load_from_memory(image_bytes).unwrap();

        Self {
            // 로드된 이미지로 Raytracer를 즉시 생성
            raytracer: Raytracer::new(1280, 720, Some(image)),
            is_first_frame: true,
        }
    }
}

impl TemplateApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        if let Some(storage) = cc.storage {
            return eframe::get_value(storage, eframe::APP_KEY)
                .unwrap_or_default();
        }
        Default::default()
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
        egui::CentralPanel::default().show(ctx, |ui| {
            let available_size = ui.available_size();
            self.raytracer.width = available_size.x as i32;
            self.raytracer.height = available_size.y as i32;

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
                ui.image((
                    texture_handle.id(),
                    texture_handle.size_vec2(),
                ));
            }
        });
    }
}
