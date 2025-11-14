use egui::{Color32, TextureOptions};
use glam::{Vec2, Vec3, vec2, vec3};
use std::any::Any;
use std::sync::Arc;

// C++의 Object 클래스 -> Rust의 Object 트레잇
// Send + Sync는 여러 스레드에서 안전하게 공유 가능하게 함 (rayon 병렬 처리용)
// Any는 런타임에 타입 정보를 제공해서 다운캐스팅이 가능하도록 함
trait Object: Send + Sync + Any {
    fn check_ray_collision(&self, ray: &Ray) -> Hit;
    fn ambient(&self) -> Vec3;
    fn diffuse(&self) -> Vec3;
    fn specular(&self) -> Vec3;
    fn alpha(&self) -> f32;
    fn as_any(&self) -> &dyn Any;
}

// C++의 Hit 구조체 -> Rust의 Hit 구조체
struct Hit {
    distance: f32,
    point: Vec3,
    normal: Vec3,
    w: Vec2, // 무게중심 좌표(Barycentric coordinates)
    object: Option<Arc<dyn Object>>,
}

// C++의 Ray 구조체 -> Rust의 Ray 구조체
struct Ray {
    origin: Vec3,
    direction: Vec3,
}

// C++의 Sphere 클래스 -> Rust의 Sphere 구조체
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
        let b = 2.0 * ray.direction.dot(oc);
        let c = oc.length_squared() - self.radius * self.radius;
        let discriminant = b * b - 4.0 * c;

        if discriminant < 0.0 {
            return Hit {
                distance: -1.0,
                point: Vec3::ZERO,
                normal: Vec3::ZERO,
                w: Vec2::ZERO,
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
                w: Vec2::ZERO,
                object: None,
            };
        }

        let point = ray.origin + ray.direction * distance;
        let normal = (point - self.center).normalize();
        Hit {
            distance,
            point,
            normal,
            w: Vec2::ZERO, // 구는 무게중심 좌표가 필요 없음
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
}

// C++의 Triangle 클래스 -> Rust의 Triangle 구조체
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
    // 광선과 산각형의 교점을 찾는 함수
    fn intersect_ray_triangle(
        &self,
        ray: &Ray,
    ) -> Option<(f32, Vec3, Vec3, f32, f32)> {
        // 광원과 삼각형 사이의 거리를 구하기 위해
        // 삼각형 평면의 수직벡터 찾기
        let face_normal =
            (self.v1 - self.v0).cross(self.v2 - self.v0).normalize();

        // back-face culling(뒷면제거): 두번그릴 필요 없으니까
        // 광선이 삼각형의 뒷면에서 온다면 충돌하지 않은것으로 처리
        if (-ray.direction).dot(face_normal) < 0.0 {
            return None; // Backface culling
        }

        // 광선과 평면이 거의 평행한 경우, 충돌하지 않음
        // (0으로 나누면 에러나는것 방지)
        if ray.direction.dot(face_normal).abs() < 1e-2 {
            return None;
        }

        // 1.광선과 삼각형이 있는 평면이 만나는 점과의 거리 t 찾기
        let t = (self.v0.dot(face_normal)
            - ray.origin.dot(face_normal))
            / ray.direction.dot(face_normal);

        // 삼각형이 시점보다 뒤에 있다면 충돌하지 않도록 처리
        if t < 0.0 {
            return None;
        }

        // 2.교점이 삼각형 내부에 있는지 확인(inside-outside test)
        let point = ray.origin + t * ray.direction;

        // 각 변에서 교점으로 향하는 벡터를 계산
        // 오른손법칙에 의해 삼각형 내부에 있다면 수직벡터는 0보다 큼
        let cross0 = (point - self.v2).cross(self.v1 - self.v2);
        let cross1 = (point - self.v0).cross(self.v2 - self.v0);
        let cross2 = (self.v1 - self.v0).cross(point - self.v0);

        // 벡터가 face_normal과 같은 방향인지 확인
        if cross0.dot(face_normal) < 0.0
            || cross1.dot(face_normal) < 0.0
            || cross2.dot(face_normal) < 0.0
        {
            return None;
        }

        /*
            무게중심 좌표(Barycentric Coordinates) 계산

            외적(cross product)의 길이는 두 벡터가 이루는 평행사변형의 넓이와 같다
            그러므로 0.5를 곱해주면 삼각형의 넓이가 된다.

            큰삼각형에서 점 p로 나눠진 작은 삼각형 세개의 비율로
            무게중심 w0, w1, w2를 알 수 있고
            색상을 인터폴레이션 하고싶다면 w0, w1, w2를 가중치로 사용해서
            점의 색깔 c를 결정할 수 있다
        */
        let area0 = cross0.length() * 0.5;
        let area1 = cross1.length() * 0.5;
        let area2 = cross2.length() * 0.5;
        let area_sum = area0 + area1 + area2;

        let w0 = area0 / area_sum;
        let w1 = area1 / area_sum;
        // w2는 1 - w0 - w1로 계산가능

        // 모든 테스트를 통과하면 교점은 삼각형 내부에 있음
        Some((t, point, face_normal, w0, w1))
    }
}

impl Object for Triangle {
    fn check_ray_collision(&self, ray: &Ray) -> Hit {
        if let Some((t, point, normal, w0, w1)) =
            self.intersect_ray_triangle(ray)
        {
            Hit {
                distance: t,
                point,
                normal,
                w: vec2(w0, w1),
                object: None,
            }
        } else {
            Hit {
                distance: -1.0,
                point: Vec3::ZERO,
                normal: Vec3::ZERO,
                w: Vec2::ZERO,
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
}

// C++의 Light 클래스 -> Rust의 Light 구조체
struct Light {
    pos: Vec3,
}

// C++의 Raytracer 클래스 -> Rust의 Raytracer 구조체
struct Raytracer {
    width: i32,
    height: i32,
    objects: Vec<Arc<dyn Object>>,
    temp_object_index: Option<usize>, // 색상 보간을 할 삼각형의 인덱스
    light: Light,
}

impl Raytracer {
    fn new(width: i32, height: i32) -> Self {
        let sphere1 = Arc::new(Sphere {
            center: vec3(1.0, 0.0, 1.5),
            radius: 0.4,
            amb: vec3(0.2, 0.2, 0.2),
            diff: vec3(1.0, 0.2, 0.2),
            spec: vec3(0.5, 0.5, 0.5),
            alpha: 10.0,
        });

        let triangle = Arc::new(Triangle {
            v0: vec3(-2.0, -2.0, 2.0),
            v1: vec3(-2.0, 2.0, 2.0),
            v2: vec3(2.0, 2.0, 2.0),
            amb: vec3(1.0, 1.0, 1.0),
            diff: vec3(0.0, 0.0, 0.0),
            spec: vec3(0.0, 0.0, 0.0),
            alpha: 10.0,
        });

        let objects: Vec<Arc<dyn Object>> = vec![sphere1, triangle];

        Self {
            width,
            height,
            temp_object_index: Some(1), // triangle이 1번 인덱스
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
            w: Vec2::ZERO,
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
            let mut color = obj.ambient();

            // C++ 코드의 tempObject 처리를 Rust 스타일로 변경
            if self.temp_object_index.is_some()
                && Arc::ptr_eq(
                    &obj,
                    &self.objects[self.temp_object_index.unwrap()],
                )
            {
                let color0 = vec3(1.0, 0.0, 0.0);
                let color1 = vec3(0.0, 1.0, 0.0);
                let color2 = vec3(0.0, 0.0, 1.0);

                let w0 = hit.w.x;
                let w1 = hit.w.y;
                let w2 = 1.0 - w0 - w1;

                color = color0 * w0 + color1 * w1 + color2 * w2;
            }

            let dir_to_light =
                (self.light.pos - hit.point).normalize();

            // 그림자 효과는 일단 주석 처리 (C++ 코드와 동일하게)
            // let shadow_ray = Ray { origin: hit.point + dir_to_light * 1e-4, direction: dir_to_light };
            // if self.find_closest_collision(&shadow_ray).distance < 0.0 {
            let diff = hit.normal.dot(dir_to_light).max(0.0);
            let reflect_dir =
                2.0 * hit.normal.dot(dir_to_light) * hit.normal
                    - dir_to_light;
            let specular = (-ray.direction)
                .dot(reflect_dir)
                .max(0.0)
                .powf(obj.alpha());

            color += obj.diffuse() * diff + obj.specular() * specular;
            // }

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
    is_first_frame: bool, // 첫 프레임에만 렌더링하기 위한 플래그
}

impl Default for TemplateApp {
    fn default() -> Self {
        Self {
            raytracer: Raytracer::new(1280, 720), // 해상도 조절 가능
            is_first_frame: true,
        }
    }
}

impl TemplateApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // 저장된 상태를 불러오거나 기본값으로 시작
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

            // C++ 코드의 count와 같이 첫 프레임에만 렌더링하도록 구현
            if self.is_first_frame {
                let mut pixels: Vec<Color32> =
                    vec![Color32::BLACK; width * height];
                self.raytracer.render(&mut pixels);

                let image = egui::ColorImage::from_rgba_unmultiplied(
                    [width, height],
                    bytemuck::cast_slice(&pixels),
                );

                // 렌더링된 이미지를 텍스처로 저장하여 재사용
                ctx.memory_mut(|mem| {
                    mem.data.insert_temp(
                        egui::Id::new("raytrace_texture"),
                        image,
                    )
                });

                self.is_first_frame = false;
            }

            // 저장된 텍스처를 불러와서 매 프레임 그리기
            if let Some(texture) = ctx.memory(|mem| {
                mem.data.get_temp::<egui::ColorImage>(egui::Id::new(
                    "raytrace_texture",
                ))
            }) {
                let texture_handle = ctx.load_texture(
                    "raytrace_canvas",
                    texture.clone(), // clone the image to create a texture
                    TextureOptions::NEAREST,
                );
                ui.image((
                    texture_handle.id(),
                    texture_handle.size_vec2(),
                ));
            }
        });

        // UI 변경 시 다시 그리도록 요청 (지금은 UI가 없으므로 첫 프레임 이후 다시 그리지 않음)
        // ctx.request_repaint();
    }
}
