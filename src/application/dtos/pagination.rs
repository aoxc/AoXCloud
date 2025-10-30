use serde::{Deserialize, Serialize};

/// Un DTO para representar información de paginación
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginationDto {
    /// Página actual (comienza en 0)
    pub page: usize,
    /// Tamaño de página
    pub page_size: usize,
    /// Número total de elementos
    pub total_items: usize,
    /// Número total de páginas
    pub total_pages: usize,
    /// Indica si hay una página siguiente
    pub has_next: bool,
    /// Indica si hay una página anterior
    pub has_prev: bool,
}

/// Un DTO para representar una solicitud de paginación
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginationRequestDto {
    /// Página solicitada (comienza en 0)
    #[serde(default)]
    pub page: usize,
    /// Tamaño de página solicitado
    #[serde(default = "default_page_size")]
    pub page_size: usize,
}

/// Un DTO para representar una respuesta paginada
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedResponseDto<T> {
    /// Datos en la página actual
    pub items: Vec<T>,
    /// Información de paginación
    pub pagination: PaginationDto,
}

impl Default for PaginationRequestDto {
    fn default() -> Self {
        Self {
            page: 0,
            page_size: default_page_size(),
        }
    }
}

/// Función para establecer el tamaño de página por defecto
fn default_page_size() -> usize {
    100 // Por defecto, 100 items por página
}

impl PaginationRequestDto {
    /// Calcula el offset para consultas paginadas
    pub fn offset(&self) -> usize {
        self.page * self.page_size
    }

    /// Calcula el límite para consultas paginadas
    pub fn limit(&self) -> usize {
        self.page_size
    }

    /// Valida y ajusta los parámetros de paginación
    pub fn validate_and_adjust(&self) -> Self {
        let mut page = self.page;
        let mut page_size = self.page_size;

        // Asegurar que la página sea al menos 0
        if page < 1 {
            page = 0;
        }

        // Asegurar que el tamaño de página esté entre 10 y 500
        if page_size < 10 {
            page_size = 10;
        } else if page_size > 500 {
            page_size = 500;
        }

        Self { page, page_size }
    }
}

impl<T> PaginatedResponseDto<T> {
    /// Crea una nueva respuesta paginada a partir de los datos y la información de paginación
    pub fn new(items: Vec<T>, page: usize, page_size: usize, total_items: usize) -> Self {
        let total_pages = if total_items == 0 {
            0
        } else {
            (total_items + page_size - 1) / page_size
        };

        let pagination = PaginationDto {
            page,
            page_size,
            total_items,
            total_pages,
            has_next: page < total_pages - 1,
            has_prev: page > 0,
        };

        Self { items, pagination }
    }
}
